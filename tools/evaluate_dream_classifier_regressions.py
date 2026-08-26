import json
import sys
from collections import defaultdict
from pathlib import Path


def load(path):
    return json.loads(Path(path).read_text(encoding="utf-8"))


def final_pairs(report):
    return {
        tuple(sorted((relation["source"], relation["target"])))
        for relation in report["durable_after_reopen"]["relations"]
        if relation.get("active", True)
    }


def ratio(good, total):
    return "n/a" if total == 0 else f"{100.0 * good / total:.1f}% ({good}/{total})"


def main():
    if len(sys.argv) not in (2, 3):
        raise SystemExit(
            "usage: evaluate_dream_classifier_regressions.py <report.json> "
            "[regression.json]"
        )
    report = load(sys.argv[1])
    fixture = load(
        sys.argv[2]
        if len(sys.argv) == 3
        else Path(__file__).resolve().parents[1]
        / "corpus/dream-classifier-regressions-v1.json"
    )
    present = final_pairs(report)
    totals = defaultdict(lambda: [0, 0])
    failures = []

    for case in fixture["cases"]:
        key = tuple(sorted((case["left"]["id"], case["right"]["id"])))
        actual_related = key in present
        expected_related = case["expected"] == "related"
        ok = actual_related == expected_related
        buckets = ["all", case["expected"], case["pattern"], case["confidence"]]
        for bucket in buckets:
            totals[bucket][1] += 1
            totals[bucket][0] += int(ok)
        if not ok:
            failures.append(
                f"{case['id']}:{case['pattern']} expected={case['expected']} "
                f"actual={'related' if actual_related else 'unrelated'}"
            )

    print(f"fixture={fixture['format']}")
    for bucket in ("all", "related", "unrelated", "high", "medium"):
        good, total = totals[bucket]
        print(f"{bucket}={ratio(good, total)}")
    print("patterns:")
    for bucket in sorted(
        key for key in totals if key not in {"all", "related", "unrelated", "high", "medium"}
    ):
        good, total = totals[bucket]
        print(f"  {bucket}={ratio(good, total)}")
    if failures:
        print("failures:")
        for failure in failures:
            print(f"  {failure}")


if __name__ == "__main__":
    main()
