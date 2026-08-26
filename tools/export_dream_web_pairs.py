#!/usr/bin/env python3
"""Export unique durable Dream relation pairs from a dream_tune report."""

import argparse
import json
from pathlib import Path


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("report")
    parser.add_argument("output")
    args = parser.parse_args()

    report = json.loads(Path(args.report).read_text(encoding="utf-8"))
    durable = report.get("durable_after_reopen", {})
    memories = {
        row["id"]: row
        for row in durable.get("memories", [])
        if row.get("id")
    }
    pairs = {}
    for relation in durable.get("relations", []):
        if not relation.get("active", True):
            continue
        source = relation.get("source")
        target = relation.get("target")
        kind = relation.get("kind")
        if not source or not target or not kind:
            continue
        key = tuple(sorted((source, target)))
        pairs.setdefault(key, set()).add(kind)

    rows = []
    for (left, right), kinds in pairs.items():
        a = memories[left]
        b = memories[right]
        rows.append(
            {
                "a": left,
                "a_title": a["title"],
                "a_content": a["content"],
                "b": right,
                "b_title": b["title"],
                "b_content": b["content"],
                "kinds": sorted(kinds),
            }
        )
    rows.sort(key=lambda row: (row["kinds"], row["a_title"], row["b_title"]))
    Path(args.output).write_text(json.dumps(rows, indent=2), encoding="utf-8")
    print(f"pairs={len(rows)}")


if __name__ == "__main__":
    main()
