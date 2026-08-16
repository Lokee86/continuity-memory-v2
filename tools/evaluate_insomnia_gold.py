#!/usr/bin/env python3
"""Score Insomnia memory dumps against the current tracked gold set.

Selection, authority provenance, grounding provenance, metadata, forbidden
content, and receipt cleanup are mechanical. semantic_target equivalence remains
a human/model judgment rather than a string-match metric.
"""

import json
import re
import sys
from collections import defaultdict
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
GOLD_PATH = ROOT / "corpus" / "insomnia-gold-v2.json"
NUMBERED_PROGRESS = re.compile(
    r"\b(?:prompts?|phases?|steps?)\s+(?:\w+\s+){0,3}\d+[a-z]*\b.*"
    r"\b(?:complete|completed|done|finished|passed|reached|checkpoint|"
    r"milestone|progress|through)\b|\bup to\s+(?:prompts?|phases?|steps?)\s+\d+",
    re.IGNORECASE,
)


def load(path):
    return json.loads(Path(path).read_text(encoding="utf-8"))


def policy_ok(policy, present):
    return (policy == "required" and present) or (policy == "forbidden" and not present)


def score_run(cases, memories):
    by_source = defaultdict(list)
    for memory in memories:
        by_source[memory.get("source_node_id")].append(memory)

    result = defaultdict(int)
    failures = []
    for case in cases:
        rows = by_source.get(case["source_node_id"], [])
        selected = bool(rows)
        expected = case["retain"]
        result["cases"] += 1
        result["selection_ok"] += selected == expected
        if selected != expected:
            failures.append((case["id"], "missing" if expected else "unexpected"))
            continue
        if not selected:
            continue

        for row in rows:
            result["retained_rows"] += 1
            result["authority_ok"] += policy_ok(
                case["authority_source"], bool(row.get("content_source_node_id"))
            )
            result["grounding_ok"] += policy_ok(
                case["grounding_source"], bool(row.get("grounding_source_node_id"))
            )
            result["metadata_ok"] += (
                row.get("category") in case["acceptable_categories"]
                and row.get("type") in case["acceptable_types"]
            )
            text = f"{row.get('title', '')} {row.get('content', '')}".lower()
            content_ok = not any(term.lower() in text for term in case["forbidden_content"])
            if case["receipt_policy"] == "strip_progress":
                content_ok = content_ok and not NUMBERED_PROGRESS.search(text)
            result["content_guard_ok"] += content_ok
    return result, failures


def pct(good, total):
    return "n/a" if not total else f"{100.0 * good / total:.1f}%"


def print_score(name, result):
    print(
        f"{name}: selection={pct(result['selection_ok'], result['cases'])} "
        f"authority={pct(result['authority_ok'], result['retained_rows'])} "
        f"grounding={pct(result['grounding_ok'], result['retained_rows'])} "
        f"metadata={pct(result['metadata_ok'], result['retained_rows'])} "
        f"guards={pct(result['content_guard_ok'], result['retained_rows'])}"
    )


def main(paths):
    if not paths:
        raise SystemExit("usage: evaluate_insomnia_gold.py <memory-dump.json> [...]")
    cases = load(GOLD_PATH)["cases"]
    aggregate = defaultdict(int)
    for path in paths:
        result, failures = score_run(cases, load(path))
        for key, value in result.items():
            aggregate[key] += value
        print_score(Path(path).name, result)
        if failures:
            print("  selection failures:", ", ".join(f"{i}:{why}" for i, why in failures))
    print_score("aggregate", aggregate)
    print("semantic_target equivalence is intentionally not auto-scored")


if __name__ == "__main__":
    main(sys.argv[1:])
