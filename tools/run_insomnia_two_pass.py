#!/usr/bin/env python3
import argparse
import copy
import json
import os
import time
from concurrent.futures import ThreadPoolExecutor, as_completed
from pathlib import Path

from insomnia_two_pass_prompts import (
    LEDGER_PROMPT,
    LEDGER_SCHEMA,
    SYNTHESIS_PROMPT,
    SYNTHESIS_SCHEMA,
)
from nous_json_client import complete_json

ROOT = Path(__file__).resolve().parents[1]
DEFAULT_INPUT = ROOT / "corpus" / "insomnia-tuning-v1.jsonl"
DEFAULT_OUTPUT = ROOT / "target" / "insomnia-ox-two-pass"
DEFAULT_ENDPOINT = "https://inference-api.nousresearch.com/v1/chat/completions"
CATEGORIES = {"fact", "preference", "decision", "instruction", "relationship", "constraint", "correction", "commitment"}
TYPES = {"identity", "education", "employment", "location", "possession", "health", "finance", "schedule", "communication", "project", "process", "product", "relationship", "other"}


def load_episodes(path):
    grouped = {}
    with Path(path).open(encoding="utf-8") as handle:
        for order, line in enumerate(handle):
            row = json.loads(line)
            if row.get("kind") != "node":
                continue
            row["_order"] = order
            grouped.setdefault(row["conversation_id"], []).append(row)
    episodes = []
    for conversation_id, turns in grouped.items():
        turns.sort(key=lambda row: (row.get("timestamp_ns", 0), row["_order"]))
        for row in turns:
            row.pop("_order", None)
        episodes.append({"conversation_id": conversation_id, "turns": turns})
    episodes.sort(key=lambda episode: episode["turns"][0].get("timestamp_ns", 0))
    return episodes


def validate_ledger(episode, ledger):
    entries = ledger.get("entries")
    if not isinstance(entries, list):
        raise ValueError("ledger.entries must be an array")
    turns = {turn["id"]: turn for turn in episode["turns"]}
    user_ids = [turn["id"] for turn in episode["turns"] if turn["role"] == "user"]
    seen = set()
    for entry in entries:
        source_id = entry.get("source_node_id")
        turn = turns.get(source_id)
        if not turn or turn["role"] != "user":
            raise ValueError(f"ledger source is not an episode user turn: {source_id}")
        seen.add(source_id)
        disposition = entry.get("disposition")
        if disposition not in {"retain", "omit", "superseded"}:
            raise ValueError(f"invalid disposition for {source_id}: {disposition}")
        quote = entry.get("source_quote", "")
        if disposition != "omit" and (not quote or quote not in turn["content"]):
            raise ValueError(f"non-exact retained/superseded quote for {source_id}")
        if disposition == "retain":
            if entry.get("authority_kind") not in {"direct", "correction", "adoption", "retention"}:
                raise ValueError(f"invalid authority kind for {source_id}")
            if entry.get("category") not in CATEGORIES or entry.get("type") not in TYPES:
                raise ValueError(f"invalid retained classification for {source_id}")
            if entry["authority_kind"] == "adoption" and not entry.get("authority_source_node_id"):
                raise ValueError(f"adoption lacks authority source for {source_id}")
    missing = [source_id for source_id in user_ids if source_id not in seen]
    if missing:
        raise ValueError(f"ledger omitted {len(missing)} user turns: {missing[:5]}")


def validate_candidates(episode, ledger, result):
    candidates = result.get("candidates")
    if not isinstance(candidates, list):
        raise ValueError("synthesis.candidates must be an array")
    turns = {turn["id"]: turn for turn in episode["turns"]}
    retained = {entry["source_node_id"] for entry in ledger["entries"] if entry.get("disposition") == "retain"}
    for candidate in candidates:
        source_id = candidate.get("source_node_id")
        if source_id not in retained:
            raise ValueError(f"candidate bypassed retained ledger: {source_id}")
        turn = turns[source_id]
        if candidate.get("source_quote") not in turn["content"]:
            raise ValueError(f"candidate quote is not exact for {source_id}")
        if candidate.get("category") not in CATEGORIES or candidate.get("type") not in TYPES:
            raise ValueError(f"invalid candidate classification for {source_id}")
        if candidate.get("authority_kind") == "adoption" and not candidate.get("authority_source_node_id"):
            raise ValueError(f"candidate adoption lacks authority source for {source_id}")
    return candidates


def _write_json(path, value):
    Path(path).write_text(json.dumps(value, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")


def canonicalize_ledger_quotes(episode, ledger):
    turns = {turn["id"]: turn for turn in episode["turns"]}
    valid_authority = {"direct", "correction", "adoption", "retention"}
    for entry in ledger.get("entries", []):
        disposition = entry.get("disposition")
        if disposition == "omit":
            entry["authority_kind"] = "none"
            entry["authority_source_node_id"] = ""
            entry["grounding_source_node_id"] = ""
            entry["category"] = "none"
            entry["type"] = "none"
            entry["lifecycle"] = "none"
            entry["proposition"] = ""
            entry["source_quote"] = ""
            continue
        authority_kind = entry.get("authority_kind")
        if authority_kind not in valid_authority:
            if entry.get("authority_source_node_id"):
                entry["authority_kind"] = "adoption"
            elif entry.get("category") == "correction":
                entry["authority_kind"] = "correction"
            else:
                entry["authority_kind"] = "direct"
        if entry.get("authority_kind") in {"direct", "correction"}:
            entry["authority_source_node_id"] = ""
        source_id = entry.get("source_node_id")
        turn = turns.get(source_id)
        if entry.get("authority_kind") == "adoption" and not entry.get("authority_source_node_id") and turn:
            parent_id = turn.get("parent_id", "")
            parent = turns.get(parent_id)
            if parent and parent.get("role") == "assistant":
                entry["authority_source_node_id"] = parent_id
        if entry.get("grounding_source_node_id") == source_id:
            entry["grounding_source_node_id"] = ""
        turn = turns.get(source_id)
        if not turn:
            continue
        content = turn.get("content", "")
        quote = entry.get("source_quote", "")
        if not quote or quote not in content:
            entry["source_quote"] = content


def canonicalize_synthesis_structure(ledger, synthesis):
    retained = [entry for entry in ledger.get("entries", []) if entry.get("disposition") == "retain"]
    valid_authority = {"direct", "correction", "adoption", "retention"}
    for candidate in synthesis.get("candidates", []):
        source_id = candidate.get("source_node_id")
        matches = [entry for entry in retained if entry.get("source_node_id") == source_id]
        if not matches:
            continue

        narrowed = matches
        authority_kind = candidate.get("authority_kind")
        if authority_kind in valid_authority:
            same = [entry for entry in narrowed if entry.get("authority_kind") == authority_kind]
            if same:
                narrowed = same
        category = candidate.get("category")
        if category in CATEGORIES:
            same = [entry for entry in narrowed if entry.get("category") == category]
            if same:
                narrowed = same
        candidate_type = candidate.get("type")
        if candidate_type in TYPES:
            same = [entry for entry in narrowed if entry.get("type") == candidate_type]
            if same:
                narrowed = same

        if authority_kind not in valid_authority:
            values = {entry.get("authority_kind") for entry in narrowed if entry.get("authority_kind") in valid_authority}
            if len(values) == 1:
                candidate["authority_kind"] = values.pop()
        if category not in CATEGORIES:
            values = {entry.get("category") for entry in narrowed if entry.get("category") in CATEGORIES}
            if len(values) == 1:
                candidate["category"] = values.pop()
        if candidate_type not in TYPES:
            values = {entry.get("type") for entry in narrowed if entry.get("type") in TYPES}
            if len(values) == 1:
                candidate["type"] = values.pop()

        authority_kind = candidate.get("authority_kind")
        if authority_kind in {"direct", "correction"}:
            candidate["authority_source_node_id"] = ""
            candidate["authority_source_quote"] = ""
            candidate["authority_source_conversation_id"] = ""
        elif authority_kind == "adoption" and not candidate.get("authority_source_node_id"):
            values = {entry.get("authority_source_node_id") for entry in narrowed if entry.get("authority_source_node_id")}
            if len(values) == 1:
                candidate["authority_source_node_id"] = values.pop()

        if candidate.get("grounding_source_node_id") == source_id:
            candidate["grounding_source_node_id"] = ""
            candidate["grounding_source_quote"] = ""
            candidate["grounding_source_conversation_id"] = ""


def canonicalize_synthesis_provenance(episode, synthesis):
    turns = {turn["id"]: turn for turn in episode["turns"]}
    conversation_id = episode.get("conversation_id", "")
    for candidate in synthesis.get("candidates", []):
        for id_key, quote_key, conversation_key in (
            ("source_node_id", "source_quote", None),
            ("authority_source_node_id", "authority_source_quote", "authority_source_conversation_id"),
            ("grounding_source_node_id", "grounding_source_quote", "grounding_source_conversation_id"),
        ):
            source_id = candidate.get(id_key, "")
            if not source_id:
                candidate[quote_key] = ""
                if conversation_key:
                    candidate[conversation_key] = ""
                continue
            turn = turns.get(source_id)
            if not turn:
                continue
            content = turn.get("content", "")
            quote = candidate.get(quote_key, "")
            if not quote or quote not in content:
                candidate[quote_key] = content
            if conversation_key:
                candidate[conversation_key] = conversation_id


def ledger_schema_for_episode(episode):
    schema = copy.deepcopy(LEDGER_SCHEMA)
    user_ids = [turn["id"] for turn in episode["turns"] if turn["role"] == "user"]
    assistant_ids = [turn["id"] for turn in episode["turns"] if turn["role"] == "assistant"] + [""]
    all_ids = [turn["id"] for turn in episode["turns"]] + [""]
    props = schema["properties"]["entries"]["items"]["properties"]
    props["source_node_id"]["enum"] = user_ids
    props["authority_source_node_id"]["enum"] = assistant_ids
    props["grounding_source_node_id"]["enum"] = all_ids
    return schema


def ledger_tool_schema_for_episode(episode):
    base = ledger_schema_for_episode(episode)
    clause = copy.deepcopy(base["properties"]["entries"]["items"])
    clause["properties"].pop("source_node_id", None)
    clause["required"] = [name for name in clause["required"] if name != "source_node_id"]
    user_ids = [turn["id"] for turn in episode["turns"] if turn["role"] == "user"]
    return {
        "type": "object",
        "additionalProperties": False,
        "properties": {
            "turns": {
                "type": "object",
                "additionalProperties": False,
                "properties": {
                    source_id: {"type": "array", "minItems": 1, "maxItems": 16, "items": copy.deepcopy(clause)}
                    for source_id in user_ids
                },
                "required": user_ids,
            }
        },
        "required": ["turns"],
    }


def normalize_tool_ledger(episode, raw):
    turns = raw.get("turns")
    if not isinstance(turns, dict):
        raise ValueError("tool ledger.turns must be an object")
    entries = []
    for turn in episode["turns"]:
        if turn["role"] != "user":
            continue
        clauses = turns.get(turn["id"])
        if not isinstance(clauses, list) or not clauses:
            raise ValueError(f"tool ledger lacks clauses for {turn['id']}")
        for clause in clauses:
            entry = dict(clause)
            entry["source_node_id"] = turn["id"]
            entries.append(entry)
    return {"entries": entries}


def synthesis_schema_for_episode(episode, ledger):
    schema = copy.deepcopy(SYNTHESIS_SCHEMA)
    retained = [entry["source_node_id"] for entry in ledger["entries"] if entry.get("disposition") == "retain"]
    assistant_ids = [turn["id"] for turn in episode["turns"] if turn["role"] == "assistant"] + [""]
    all_ids = [turn["id"] for turn in episode["turns"]] + [""]
    candidate = schema["properties"]["candidates"]["items"]["properties"]
    candidate["source_node_id"]["enum"] = retained or [""]
    candidate["authority_source_node_id"]["enum"] = assistant_ids
    candidate["grounding_source_node_id"]["enum"] = all_ids
    if not retained:
        schema["properties"]["candidates"]["maxItems"] = 0
    return schema


def run_episode(index, episode, args, api_key):
    output = Path(args.output)
    ledger_path = output / f"episode-{index + 1:03d}-ledger.json"
    synthesis_path = output / f"episode-{index + 1:03d}-synthesis.json"
    usage1 = {}
    usage2 = {}

    if args.resume and ledger_path.exists():
        ledger = json.loads(ledger_path.read_text(encoding="utf-8"))
        canonicalize_ledger_quotes(episode, ledger)
        validate_ledger(episode, ledger)
    else:
        user_ids = [turn["id"] for turn in episode["turns"] if turn["role"] == "user"]
        ledger_payload = dict(episode)
        ledger_payload["required_user_turn_ids"] = user_ids
        ledger_prompt = LEDGER_PROMPT + (
            "\n\nCoverage checklist for this structured call: required_user_turn_ids is exhaustive. "
            "Every ID in that list MUST be accounted for, including pure questions, which require an explicit omit clause. "
            "When the tool schema presents a required `turns` object keyed by user-turn ID, fill every required key with "
            "one or more clause decisions for that exact turn. Do not skip any key."
        )
        ledger_schema = ledger_tool_schema_for_episode(episode) if args.structured_mode == "tool" else ledger_schema_for_episode(episode)
        raw_ledger, usage1 = complete_json(
            args.endpoint, api_key, args.model, ledger_prompt, ledger_payload,
            args.timeout, args.retries, args.reasoning_effort,
            "insomnia_authority_ledger", ledger_schema,
            structured_mode=args.structured_mode,
        )
        if args.structured_mode == "tool":
            _write_json(output / f"episode-{index + 1:03d}-ledger-raw.json", raw_ledger)
            ledger = normalize_tool_ledger(episode, raw_ledger)
        else:
            ledger = raw_ledger
        canonicalize_ledger_quotes(episode, ledger)
        _write_json(ledger_path, ledger)
        validate_ledger(episode, ledger)

    if args.resume and synthesis_path.exists():
        raw_backup = output / f"episode-{index + 1:03d}-synthesis-pre-normalize.json"
        if not raw_backup.exists():
            raw_backup.write_text(synthesis_path.read_text(encoding="utf-8"), encoding="utf-8")
        synthesis = json.loads(synthesis_path.read_text(encoding="utf-8"))
    else:
        synthesis_payload = {"authoritative_episode": episode, "authority_disposition_ledger": ledger}
        synthesis, usage2 = complete_json(
            args.endpoint, api_key, args.model, SYNTHESIS_PROMPT, synthesis_payload,
            args.timeout, args.retries, args.reasoning_effort,
            "insomnia_memory_synthesis", synthesis_schema_for_episode(episode, ledger),
            structured_mode=args.structured_mode,
        )

    canonicalize_synthesis_structure(ledger, synthesis)
    canonicalize_synthesis_provenance(episode, synthesis)
    _write_json(synthesis_path, synthesis)
    candidates = validate_candidates(episode, ledger, synthesis)
    result = {"episode": episode, "ledger": ledger, "candidates": candidates, "usage": [usage1, usage2]}
    _write_json(output / f"episode-{index + 1:03d}-result.json", result)
    return index, result


def memory_row(candidate):
    return {
        "authority_kind": candidate.get("authority_kind"),
        "category": candidate.get("category"),
        "type": candidate.get("type"),
        "title": candidate.get("title"),
        "content": candidate.get("content"),
        "source_node_id": candidate.get("source_node_id"),
        "source_quote": candidate.get("source_quote"),
        "content_source_node_id": candidate.get("authority_source_node_id") or None,
        "grounding_source_node_id": candidate.get("grounding_source_node_id") or None,
    }


def main():
    parser = argparse.ArgumentParser(description="Experimental two-pass Insomnia authority-ledger tuning run")
    parser.add_argument("--input", default=str(DEFAULT_INPUT))
    parser.add_argument("--output", default=str(DEFAULT_OUTPUT))
    parser.add_argument("--model", default="stealth/ox-alpha")
    parser.add_argument("--endpoint", default=DEFAULT_ENDPOINT)
    parser.add_argument("--workers", type=int, default=4)
    parser.add_argument("--reasoning-effort", default="low")
    parser.add_argument("--limit", type=int)
    parser.add_argument("--timeout", type=int, default=240)
    parser.add_argument("--retries", type=int, default=3)
    parser.add_argument("--structured-mode", choices=["response_format", "tool"], default="response_format")
    parser.add_argument("--env-file")
    parser.add_argument("--resume", action="store_true")
    args = parser.parse_args()
    api_key = os.environ.get("NOUS_API_KEY") or os.environ.get("RELIQUARY_INSOMNIA_API_KEY")
    if not api_key and args.env_file:
        for line in Path(args.env_file).read_text(encoding="utf-8").splitlines():
            if line.startswith("RELIQUARY_INSOMNIA_API_KEY="):
                api_key = line.split("=", 1)[1].strip().strip('"').strip("'")
                break
    if not api_key:
        raise SystemExit("set NOUS_API_KEY/RELIQUARY_INSOMNIA_API_KEY or pass --env-file")

    episodes = load_episodes(args.input)
    if args.limit is not None:
        episodes = episodes[:args.limit]
    output = Path(args.output)
    output.mkdir(parents=True, exist_ok=True)
    started = time.time()
    results = [None] * len(episodes)
    with ThreadPoolExecutor(max_workers=max(1, args.workers)) as pool:
        futures = {pool.submit(run_episode, i, episode, args, api_key): i for i, episode in enumerate(episodes)}
        for future in as_completed(futures):
            index, result = future.result()
            results[index] = result
            print(f"episode {index + 1}/{len(episodes)}: {len(result['candidates'])} candidates", flush=True)

    memories = [memory_row(candidate) for result in results for candidate in result["candidates"]]
    (output / "run.json").write_text(json.dumps(results, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    (output / "memories.json").write_text(json.dumps(memories, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    meta = {
        "model": args.model,
        "reasoning_effort": args.reasoning_effort,
        "endpoint": args.endpoint,
        "episodes": len(results),
        "memories": len(memories),
        "seconds": round(time.time() - started, 3),
    }
    (output / "meta.json").write_text(json.dumps(meta, indent=2) + "\n", encoding="utf-8")
    print(json.dumps(meta))


if __name__ == "__main__":
    main()
