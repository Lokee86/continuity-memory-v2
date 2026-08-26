#!/usr/bin/env python3
"""Deterministically score a dream_tune report against Dream gold contracts."""

import argparse
import json
from pathlib import Path


def load(path):
    return json.loads(Path(path).read_text(encoding="utf-8"))


def pct(good, total):
    return "n/a" if not total else f"{100.0 * good / total:.1f}%"


def pair_index(report):
    index = {}
    retrieval = {}
    for run in report.get("runs", []):
        source = run.get("source_before", {}).get("id")
        if not source:
            continue
        retrieval[source] = {
            item.get("id") for item in run.get("retrieval", []) if item.get("id")
        }
        result = run.get("result") or {}
        for pair in result.get("pairs", []):
            candidate = pair.get("candidate_id")
            if candidate:
                index[(source, candidate)] = pair
    return index, retrieval


def final_relation_set(report):
    durable = report.get("durable_after_reopen", {})
    return {
        (row.get("source"), row.get("target"), row.get("kind"))
        for row in durable.get("relations", [])
        if row.get("active", True)
    }


def final_memory_index(report):
    durable = report.get("durable_after_reopen", {})
    return {
        row.get("id"): row
        for row in durable.get("memories", [])
        if row.get("id")
    }


def score(report, gold):
    pairs, retrieval = pair_index(report)
    attempted_sources = {
        run.get("source_before", {}).get("id")
        for run in report.get("runs", [])
        if run.get("source_before", {}).get("id")
    }
    relations = final_relation_set(report)
    related_pairs = {(source, target) for source, target, _ in relations}
    related_pairs |= {(target, source) for source, target in list(related_pairs)}
    memories = final_memory_index(report)
    totals = {
        "retrieval": [0, 0],
        "classification": [0, 0],
        "verification": [0, 0],
        "publication": [0, 0],
        "final_relation": [0, 0],
        "final_pair": [0, 0],
        "lifecycle": [0, 0],
        "backlog": [0, 0],
    }
    failures = []

    for case in gold.get("pairs", []):
        source = case["source"]
        candidate = case["candidate"]
        if source not in attempted_sources:
            continue
        pair = pairs.get((source, candidate))
        if case.get("retrieval") == "required":
            totals["retrieval"][1] += 1
            ok = candidate in retrieval.get(source, set())
            totals["retrieval"][0] += int(ok)
            if not ok:
                failures.append(f"{case.get('id', source[:8])}: retrieval")
        if pair is not None and ("related" in case or "relation" in case):
            totals["classification"][1] += 1
            actual = pair.get("classification", {})
            if "relation" in case:
                ok = actual.get("relation") == case["relation"]
                if "direction" in case:
                    ok = ok and actual.get("direction") == case["direction"]
            else:
                ok = (actual.get("relation") not in (None, "None")) == case["related"]
            totals["classification"][0] += int(ok)
            if not ok:
                failures.append(f"{case.get('id', source[:8])}: classification")
        if pair is not None and "verdict" in case:
            totals["verification"][1] += 1
            verdict = (pair.get("verification") or {}).get("verdict")
            ok = verdict == case["verdict"]
            totals["verification"][0] += int(ok)
            if not ok:
                failures.append(f"{case.get('id', source[:8])}: verification")
        if pair is not None and "publication" in case:
            totals["publication"][1] += 1
            outcome = (pair.get("publication") or {}).get("outcome")
            ok = outcome == case["publication"]
            totals["publication"][0] += int(ok)
            if not ok:
                failures.append(f"{case.get('id', source[:8])}: publication")

    for case in ([] if report.get("retrieval_only") else gold.get("final_relations", [])):
        totals["final_relation"][1] += 1
        key = (case["source"], case["target"], case["kind"])
        expected = case.get("present", True)
        ok = (key in relations) == expected
        totals["final_relation"][0] += int(ok)
        if not ok:
            failures.append(f"{case.get('id', case['kind'])}: final_relation")

    for case in ([] if report.get("retrieval_only") else gold.get("final_pairs", [])):
        if case["left"] not in attempted_sources and case["right"] not in attempted_sources:
            continue
        totals["final_pair"][1] += 1
        key = (case["left"], case["right"])
        expected = case.get("present", True)
        ok = (key in related_pairs) == expected
        totals["final_pair"][0] += int(ok)
        if not ok:
            failures.append(f"{case.get('id', case['left'][:8])}: final_pair")

    for case in ([] if report.get("retrieval_only") else gold.get("lifecycle", [])):
        totals["lifecycle"][1] += 1
        memory = memories.get(case["id"], {})
        ok = bool(memory)
        for field in ("lifecycle", "archived"):
            if field in case:
                ok = ok and memory.get(field) == case[field]
        totals["lifecycle"][0] += int(ok)
        if not ok:
            failures.append(f"{case['id'][:8]}: lifecycle")

    baseline_extracted = report.get("baseline", {}).get("remaining_extracted")
    if (
        not report.get("retrieval_only")
        and "remaining_extracted" in gold
        and report.get("attempted") == baseline_extracted
    ):
        totals["backlog"][1] = 1
        actual = report.get("durable_after_reopen", {}).get("remaining_extracted")
        ok = actual == gold["remaining_extracted"]
        totals["backlog"][0] = int(ok)
        if not ok:
            failures.append("remaining_extracted: backlog")

    return totals, failures


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("report")
    parser.add_argument("gold")
    args = parser.parse_args()
    report = load(args.report)
    gold = load(args.gold)
    totals, failures = score(report, gold)
    print(f"gold={gold.get('format', Path(args.gold).name)}")
    for name, (good, total) in totals.items():
        print(f"{name}={pct(good, total)} ({good}/{total})")
    if failures:
        print("failures: " + ", ".join(failures))
    print(
        "run: attempted={} recoverable_failures={} remaining_extracted={}".format(
            report.get("attempted", 0),
            report.get("recoverable_failures", 0),
            report.get("durable_after_reopen", {}).get("remaining_extracted", "?"),
        )
    )


if __name__ == "__main__":
    main()
