import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
V1_REPORT = ROOT / "target/dream-tune-full-20260826-codex-sol-low-v2/report.json"
V2_REPORT = ROOT / "target/dream-tune-full-20260826-codex-sol-low-classifier-v2/report.json"
GOLD = ROOT / "corpus/dream-web-gold-v1.json"
AUDIT = ROOT / "corpus/dream-web-audit-v1.json"
OUTPUT = ROOT / "corpus/dream-classifier-regressions-v1.json"

POSITIVE = {
    "web-010": "packet_workstream",
    "web-021": "status_summary_detail",
    "web-023": "packet_workstream",
    "web-029": "workspace_operational",
    "web-031": "workspace_operational",
    "web-032": "packet_workstream",
    "web-034": "interaction_preference",
    "web-042": "multiplayer_flow",
    "web-043": "multiplayer_flow",
    "web-046": "packet_workstream",
    "web-050": "packet_workstream",
    "web-051": "packet_workstream",
    "web-054": "multiplayer_flow",
    "web-058": "conceptual_analogy",
    "web-061": "multiplayer_flow",
    "web-066": "workspace_operational",
    "web-067": "packet_workstream",
    "web-069": "packet_workstream",
    "web-073": "ship_runtime_identity",
    "web-074": "network_protocol",
    "web-078": "packet_workstream",
    "web-081": "ship_runtime_identity",
    "web-082": "ship_runtime_identity",
    "web-086": "asteroid_lifecycle",
    "web-089": "interaction_preference",
    "web-093": "interaction_preference",
    "web-094": "interaction_preference",
    "web-100": "network_protocol",
    "web-103": "workspace_operational",
    "web-missing-troubleshooting-style": "interaction_preference",
}

NEGATIVE = {
    "web-019": "cross_project_history",
    "web-038": "cross_project_history",
    "web-039": "cross_project_history",
    "web-045": "cross_project_history",
    "web-080": "cross_subsystem_same_project",
    "web-083": "cross_subsystem_same_project",
    "web-088": "cross_subsystem_same_project",
    "web-090": "cross_project_history",
    "web-092": "cross_project_history",
    "web-099": "lexical_collision_cross_entity",
}

LOWER_CONFIDENCE = {
    "web-029",
    "web-031",
    "web-073",
    "web-081",
    "web-082",
    "web-089",
    "web-103",
}


def load(path):
    return json.loads(path.read_text(encoding="utf-8"))


def relation_map(report):
    out = {}
    for relation in report["durable_after_reopen"]["relations"]:
        key = tuple(sorted((relation["source"], relation["target"])))
        out.setdefault(key, relation["kind"])
    return out


def v2_evaluations(report, left, right):
    evaluations = []
    for run in report["runs"]:
        source = run["source_before"]["id"]
        for pair in (run.get("result") or {}).get("pairs", []):
            if {source, pair["candidate_id"]} != {left, right}:
                continue
            evaluations.append({
                "source": source,
                "source_title": run["source_before"]["title"],
                "relation": pair["classification"]["relation"],
                "direction": pair["classification"]["direction"],
                "publication": (pair.get("publication") or {}).get("outcome"),
            })
    return evaluations


def main():
    v1 = load(V1_REPORT)
    v2 = load(V2_REPORT)
    gold = load(GOLD)
    audit = load(AUDIT)
    memories = {item["id"]: item for item in v1["durable_after_reopen"]["memories"]}
    v1_relations = relation_map(v1)
    v2_relations = relation_map(v2)
    gold_cases = {item["id"]: item for item in gold["final_pairs"]}
    audit_cases = {f"web-{item['index']:03d}": item for item in audit["pairs"]}

    cases = []
    for case_id, pattern in {**POSITIVE, **NEGATIVE}.items():
        gold_case = gold_cases[case_id]
        left = gold_case["left"]
        right = gold_case["right"]
        key = tuple(sorted((left, right)))
        expected_related = case_id in POSITIVE
        audit_case = audit_cases.get(case_id)
        cases.append({
            "id": case_id,
            "pattern": pattern,
            "expected": "related" if expected_related else "unrelated",
            "confidence": "medium" if case_id in LOWER_CONFIDENCE else "high",
            "left": {"id": left, "title": memories[left]["title"], "content": memories[left]["content"]},
            "right": {"id": right, "title": memories[right]["title"], "content": memories[right]["content"]},
            "v1_final_kind": v1_relations.get(key),
            "v2_final_kind": v2_relations.get(key),
            "v2_evaluations": v2_evaluations(v2, left, right),
            "audit_reason": audit_case.get("reason") if audit_case else None,
        })

    output = {
        "format": "dream-classifier-regressions-v1",
        "purpose": "Boundary fixture for classifier tuning after v2 over-pruned legitimate same-workstream relations.",
        "rule": "Related pairs share a specific semantic workstream or explanatory scope; unrelated pairs only share project/domain/vocabulary context across workstreams or project identities.",
        "summary": {
            "cases": len(cases),
            "positive": len(POSITIVE),
            "negative": len(NEGATIVE),
            "medium_confidence_positive": len(LOWER_CONFIDENCE),
        },
        "cases": cases,
    }
    OUTPUT.write_text(json.dumps(output, indent=2) + "\n", encoding="utf-8")
    print(f"wrote {OUTPUT.name}: {len(cases)} cases")


if __name__ == "__main__":
    main()
