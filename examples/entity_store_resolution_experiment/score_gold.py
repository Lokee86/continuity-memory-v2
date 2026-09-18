import json
import sys
from collections import defaultdict


def load_results(path):
    with open(path, encoding="utf-8") as stream:
        return [json.loads(line) for line in stream if line.strip()]


results = load_results(sys.argv[1])
with open(sys.argv[2], encoding="utf-8") as stream:
    gold = json.load(stream)

DECISIONS = ("resolve_existing", "create_new", "unresolved", "reject")

def expected_for(row):
    value = gold[row["memory_id"]]
    return {"decision": value["expected_decision"], "entity": value.get("expected_entity_id"),
            "category": value.get("category", "uncategorized")}


def is_correct(got, expected):
    return (got.get("decision") == expected["decision"] and
            (expected["decision"] != "resolve_existing" or got.get("target_entity_id") == expected["entity"]))


def counts():
    return {"queries": 0, "correct": 0, "permutations": 0, "permutation_correct": 0}

by_decision = defaultdict(counts)
by_category = defaultdict(counts)
metrics = {name: {"query": 0, "permutation": 0} for name in
           ("false_merge", "false_new", "over_unresolved", "false_reject")}
failures = []
query_correct = permutation_correct = permutation_total = invariant = 0

for row in results:
    expected = expected_for(row)
    groups = (by_decision[expected["decision"]], by_category[expected["category"]])
    for group in groups:
        group["queries"] += 1
    permutations = row.get("permutation_results", [])
    permutation_total += len(permutations)
    invariant += int(bool(row.get("order_invariant")))
    oks = []
    query_flags = set()
    for result in permutations:
        ok = is_correct(result, expected)
        oks.append(ok)
        permutation_correct += int(ok)
        got = result.get("decision")
        for group in groups:
            group["permutations"] += 1
            group["permutation_correct"] += int(ok)
        if got == "resolve_existing" and (expected["decision"] != "resolve_existing" or result.get("target_entity_id") != expected["entity"]):
            query_flags.add("false_merge"); metrics["false_merge"]["permutation"] += 1
        if got == "create_new" and expected["decision"] == "resolve_existing":
            query_flags.add("false_new"); metrics["false_new"]["permutation"] += 1
        if got == "unresolved" and expected["decision"] in ("resolve_existing", "create_new", "reject"):
            query_flags.add("over_unresolved"); metrics["over_unresolved"]["permutation"] += 1
        if got == "reject" and expected["decision"] in ("resolve_existing", "create_new", "unresolved"):
            query_flags.add("false_reject"); metrics["false_reject"]["permutation"] += 1
    full = bool(oks) and all(oks)
    query_correct += int(full)
    for group in groups:
        group["correct"] += int(full)
    for flag in query_flags:
        metrics[flag]["query"] += 1
    if not full:
        failures.append({"memory_id": row["memory_id"], "category": expected["category"],
                         "expected": expected, "order_invariant": row.get("order_invariant"),
                         "outcomes": [{k: result.get(k) for k in ("entity_id_order", "decision", "target_entity_id", "reason", "error")} for result in permutations]})


def finish(group):
    group["query_accuracy"] = group["correct"] / group["queries"] if group["queries"] else 1.0
    group["permutation_accuracy"] = group["permutation_correct"] / group["permutations"] if group["permutations"] else 1.0
    return group

print(json.dumps({
    "total_queries": len(results), "query_consensus_full_correct": query_correct,
    "query_consensus_full_accuracy": query_correct / len(results) if results else 1.0,
    "permutation_correct": permutation_correct, "permutation_total": permutation_total,
    "permutation_accuracy": permutation_correct / permutation_total if permutation_total else 1.0,
    "by_expected_decision": {key: finish(value) for key, value in sorted(by_decision.items())},
    "by_gold_category": {key: finish(value) for key, value in sorted(by_category.items())},
    "false_merge_count": metrics["false_merge"], "false_new_count": metrics["false_new"],
    "over_unresolved_count": metrics["over_unresolved"], "false_reject_count": metrics["false_reject"],
    "order_invariant_count": invariant, "order_invariance_rate": invariant / len(results) if results else 1.0,
    "failures": failures,
}, indent=2))
