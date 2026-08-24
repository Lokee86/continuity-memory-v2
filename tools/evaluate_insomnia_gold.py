#!/usr/bin/env python3
"""Score Insomnia memory dumps against tracked gold contracts.

Gold v3 separates exact benchmark-anchor disposition from semantic-state source
coverage. Provenance, metadata, forbidden-content, and receipt guards remain
mechanical; semantic_target equivalence still requires human/model judgment.

When a memory dump has a sibling run.json, scoring is automatically scoped to
source turns that were actually presented in that run. This prevents trimmed
fixtures from being penalized for gold cases they could not possibly emit.
"""

import argparse
import json
import re
from collections import defaultdict
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
DEFAULT_GOLD = ROOT / "corpus" / "insomnia-gold-v3.json"
NUMBERED_PROGRESS = re.compile(
    r"\b(?:prompts?|phases?|steps?)\s+(?:\w+\s+){0,3}\d+[a-z]*\b.*"
    r"\b(?:complete|completed|done|finished|passed|reached|checkpoint|"
    r"milestone|progress|through)\b|\bup to\s+(?:prompts?|phases?|steps?)\s+\d+",
    re.IGNORECASE,
)


def load(path):
    return json.loads(Path(path).read_text(encoding="utf-8"))


def available_sources(path):
    run_path = Path(path).with_name("run.json")
    if not run_path.exists():
        return None
    sources = set()
    for episode in load(run_path):
        for entry in episode.get("ledger", {}).get("entries", []):
            source = entry.get("source_node_id")
            if source:
                sources.add(source)
    return sources


def policy_ok(policy, present):
    if policy in (None, "not_applicable", "optional"):
        return True
    return (policy == "required" and present) or (policy == "forbidden" and not present)


def source_contract(case, source):
    merged = dict(case)
    merged.update(source)
    return merged


def normalized_sources(case):
    sources = case.get("coverage_sources")
    if sources is not None:
        return sources
    if case.get("retain"):
        return [{"source_node_id": case["source_node_id"], "relation": "anchor"}]
    return []


def anchor_disposition(case):
    return case.get("anchor_disposition", "retain" if case.get("retain") else "omit")


def state_retain(case):
    return case.get("state_retain", bool(case.get("retain")))


def metadata_ok(contract, row):
    return (
        row.get("category") in contract.get("acceptable_categories", [])
        and row.get("type") in contract.get("acceptable_types", [])
    )


def row_guard_ok(contract, row):
    text = f"{row.get('title', '')} {row.get('content', '')}".lower()
    if any(term.lower() in text for term in contract.get("forbidden_content", [])):
        return False
    if contract.get("receipt_policy") == "strip_progress" and NUMBERED_PROGRESS.search(text):
        return False
    return True


def coverage_rows(contract, rows):
    categories = contract.get("coverage_categories")
    types = contract.get("coverage_types")
    return [
        row
        for row in rows
        if (not categories or row.get("category") in categories)
        and (not types or row.get("type") in types)
    ]


def score_source(contract, rows, result, failures, case_id, relation):
    selected = coverage_rows(contract, rows)
    if not selected:
        return False
    result["source_contracts"] += 1
    source_metadata_ok = any(metadata_ok(contract, row) for row in selected)
    result["metadata_ok"] += source_metadata_ok
    if not source_metadata_ok:
        failures["metadata"].append((case_id, relation))

    for row in selected:
        result["contract_rows"] += 1
        authority_ok = policy_ok(
            contract.get("authority_source"), bool(row.get("content_source_node_id"))
        )
        grounding_ok = policy_ok(
            contract.get("grounding_source"), bool(row.get("grounding_source_node_id"))
        )
        guard_ok = row_guard_ok(contract, row)
        result["authority_ok"] += authority_ok
        result["grounding_ok"] += grounding_ok
        result["content_guard_ok"] += guard_ok
        if not authority_ok:
            failures["authority"].append((case_id, relation))
        if not grounding_ok:
            failures["grounding"].append((case_id, relation))
        if not guard_ok:
            failures["guards"].append((case_id, relation))
    return True


def source_is_available(source_node_id, scope):
    return scope is None or source_node_id in scope


def score_run(cases, memories, scope=None):
    by_source = defaultdict(list)
    for memory in memories:
        by_source[memory.get("source_node_id")].append(memory)

    result = defaultdict(int)
    failures = defaultdict(list)
    for case in cases:
        anchor_source = case["source_node_id"]
        anchor_rows = by_source.get(anchor_source, [])

        if source_is_available(anchor_source, scope):
            disposition = anchor_disposition(case)
            anchor_expected = disposition == "retain"
            anchor_ok = bool(anchor_rows) == anchor_expected
            result["cases"] += 1
            result["anchor_ok"] += anchor_ok
            if not anchor_ok:
                why = "missing" if anchor_expected else f"unexpected_{disposition}"
                failures["anchor"].append((case["id"], why))

        sources = [
            source
            for source in normalized_sources(case)
            if source_is_available(source["source_node_id"], scope)
        ]
        valid_coverage = False
        for source in sources:
            contract = source_contract(case, source)
            relation = source.get("relation", "anchor")
            valid_coverage = score_source(
                contract,
                by_source.get(source["source_node_id"], []),
                result,
                failures,
                case["id"],
                relation,
            ) or valid_coverage

        if state_retain(case):
            if sources:
                result["state_cases"] += 1
                result["state_coverage_ok"] += valid_coverage
                if not valid_coverage:
                    failures["coverage"].append((case["id"], "missing_valid_state"))
        elif source_is_available(anchor_source, scope):
            result["omit_cases"] += 1
            clean = not anchor_rows
            result["omit_ok"] += clean
            if not clean:
                failures["omit"].append((case["id"], "unexpected_state"))
    return result, failures


def pct(good, total):
    return "n/a" if not total else f"{100.0 * good / total:.1f}%"


def print_score(name, result):
    print(
        f"{name}: anchor={pct(result['anchor_ok'], result['cases'])} "
        f"state_coverage={pct(result['state_coverage_ok'], result['state_cases'])} "
        f"omit={pct(result['omit_ok'], result['omit_cases'])} "
        f"authority={pct(result['authority_ok'], result['contract_rows'])} "
        f"grounding={pct(result['grounding_ok'], result['contract_rows'])} "
        f"metadata={pct(result['metadata_ok'], result['source_contracts'])} "
        f"guards={pct(result['content_guard_ok'], result['contract_rows'])}"
    )


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("paths", nargs="+")
    parser.add_argument("--gold", default=str(DEFAULT_GOLD))
    parser.add_argument(
        "--unscoped",
        action="store_true",
        help="score all gold cases even when a sibling run.json identifies a trimmed fixture",
    )
    args = parser.parse_args()

    gold = load(args.gold)
    cases = gold["cases"]
    aggregate = defaultdict(int)
    print(f"gold={gold.get('version', Path(args.gold).name)}")
    for path in args.paths:
        scope = None if args.unscoped else available_sources(path)
        result, failures = score_run(cases, load(path), scope)
        for key, value in result.items():
            aggregate[key] += value
        print_score(Path(path).name, result)
        if scope is not None:
            print(
                f"  scoped to {len(scope)} presented user turns: "
                f"{result['cases']} anchor cases, {result['state_cases']} state cases, "
                f"{result['omit_cases']} omit cases"
            )
        for kind in (
            "anchor",
            "coverage",
            "omit",
            "authority",
            "grounding",
            "metadata",
            "guards",
        ):
            if failures[kind]:
                detail = ", ".join(f"{case}:{why}" for case, why in failures[kind])
                print(f"  {kind} failures: {detail}")
    print_score("aggregate", aggregate)
    print("semantic_target equivalence remains intentionally manual/model-judged")


if __name__ == "__main__":
    main()
