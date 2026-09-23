#!/usr/bin/env python3
from __future__ import annotations
import csv, json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
FIX = ROOT / "fixtures" / "local" / "calibration"
OUT = ROOT / "corpus" / "gpt56-vs-gpt6-e2e-v1"

INSOMNIA_56_CHAIN = [
    {"stage":"semantic_ledger","model":"gpt-5.6-sol","reasoning":"low"},
    {"stage":"metadata_classification","model":"gpt-5.6-luna","reasoning":"low"},
    {"stage":"ownership_classification","model":"gpt-5.6-luna","reasoning":"low","route":"insomnia_metadata"},
    {"stage":"wording","model":"gpt-5.6-sol","reasoning":"low"},
]
INSOMNIA_6_CHAIN = [
    {"stage":"semantic_ledger","model":"gpt-6-sol","reasoning":"low"},
    {"stage":"metadata_classification","model":"gpt-6-luna","reasoning":"low"},
    {"stage":"ownership_classification","model":"gpt-6-luna","reasoning":"low","route":"insomnia_metadata"},
    {"stage":"wording","model":"gpt-6-sol","reasoning":"low"},
]

def j(path):
    return json.loads(path.read_text(encoding="utf-8"))

def jl(path):
    return [json.loads(x) for x in path.read_text(encoding="utf-8").splitlines() if x.strip()]

def dumpj(path, value):
    path.write_text(json.dumps(value, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")

def dumpjl(path, rows):
    path.write_text("".join(json.dumps(x, ensure_ascii=False, separators=(",", ":")) + "\n" for x in rows), encoding="utf-8")

def take_by(rows, key, counts):
    out = []
    for value, count in counts:
        out += [r for r in rows if key(r) == value and r not in out][:count]
    return out

OUT.mkdir(parents=True, exist_ok=True)

# Insomnia: compact subset of the 11-Episode tuning corpus, preserving exact Episode context.
ig = j(ROOT / "corpus" / "insomnia-gold-v3-tuning-v1.json")
nodes = jl(ROOT / "corpus" / "insomnia-tuning-v1.jsonl")
with (ROOT / "corpus" / "insomnia-sol-low-semantic-audit-v1.csv").open(encoding="utf-8", newline="") as f:
    audit = {r["case_id"]: r for r in csv.DictReader(f)}
wanted = [
    "concise-responses", "trades-experience", "creatureserver-mothballed",
    "local-spawner-question", "collision-authority-narrow", "godot-first-question",
    "workspace-repository-location", "deleted-assets-contextual-correction",
    "adopt-c-family-adapter", "adopt-c-calibration-corpus",
]
by_node = {n["id"]: n for n in nodes}
by_conv = {}
for n in nodes:
    by_conv.setdefault(n["conversation_id"], []).append(n)
insomnia, episode_ids = [], set()
def add_insomnia(c):
    source = by_node[c["source_node_id"]]
    episode_ids.add(source["conversation_id"])
    insomnia.append({
        "id": c["id"], "source_node_id": c["source_node_id"],
        "conversation_id": source["conversation_id"], "gold": c,
        "baseline_chain": INSOMNIA_56_CHAIN, "comparison_chain": INSOMNIA_6_CHAIN,
        "baseline_5_6_runs": {k: audit[c["id"]][k] for k in ("run1","run2","run3")},
        "baseline_note": audit[c["id"]]["note"],
    })
for c in ig["cases"]:
    if c["id"] in wanted and c["id"] in audit and c["source_node_id"] in by_node:
        add_insomnia(c)
dumpjl(OUT / "insomnia_cases.jsonl", insomnia)
dumpjl(OUT / "insomnia_episodes.jsonl", [
    {"conversation_id": cid, "turns": by_conv[cid]} for cid in sorted(episode_ids)
])

# Dream: production comparison is Luna-low. The regression corpus also retains an older
# Sol-low case-level control, but the accepted v2f Luna evidence is aggregate (37/40,
# 39/40, 38/40 exact-pair runs; 96/98 population agreement), not per-case output.
dream_all = j(ROOT / "corpus" / "dream-classifier-regressions-v1.json")["cases"]
dream = take_by(dream_all, lambda r: (r["expected"], r["confidence"]), [
    (("related","high"), 5), (("related","medium"), 1),
    (("unrelated","high"), 3), (("unrelated","medium"), 1),
])
for r in dream_all:
    if len(dream) >= 10: break
    if r not in dream: dream.append(r)
for r in dream:
    r["baseline_model"] = "gpt-5.6-luna"
    r["comparison_model"] = "gpt-6-luna"
    r["reasoning"] = "low"
    r["gold"] = {"expected": r["expected"], "confidence": r["confidence"]}
    r["baseline_5_6"] = {
        "kind":"historical_aggregate",
        "exact_pair_runs":["37/40","39/40","38/40"],
        "population_agreement":"96/98",
        "case_level_output_preserved":False,
    }
    r["legacy_sol_control"] = {"final_kind": r.get("v1_final_kind")}
dumpjl(OUT / "dream_cases.jsonl", dream)

# Chronos: select a tiny structural cross-section from the final pre-GPT-6 calibration snapshot.
chronos_all = jl(ROOT / "target" / "chronos-28d-rel-final.jsonl")
chronos, seen = [], set()
for kind in ("Explicit","Relative","Calendar","Duration","Recurrence","Boundary"):
    for r in chronos_all:
        if r.get("memory_id") in seen:
            continue
        if any(x.get("kind") == kind for x in r.get("indications", [])):
            chronos.append(r); seen.add(r.get("memory_id")); break
for pred in (
    lambda r: r.get("anchors",0) > 0,
    lambda r: r.get("intervals",0) > 0,
    lambda r: bool(r.get("unresolved")),
):
    for r in chronos_all:
        if len(chronos) >= 8: break
        if r.get("memory_id") not in seen and pred(r):
            chronos.append(r); seen.add(r.get("memory_id")); break
for r in chronos_all:
    if len(chronos) >= 8: break
    if r.get("memory_id") not in seen and r.get("indications"):
        chronos.append(r); seen.add(r.get("memory_id"))
for r in chronos[:8]:
    r["baseline_route"] = {"model":"gpt-5.6-sol","reasoning":"low","via":"historical chronos->insomnia fallback"}
    r["comparison_route"] = {"model":"gpt-6-sol","reasoning":"low","via":"dedicated chronos route"}
dumpjl(OUT / "chronos_cases.jsonl", chronos[:8])

# Entity mention extraction: exact 5.6 output + gold + source Memory.
eg = {r["memory_id"]: r for r in jl(FIX / "insomnia-entity-v2" / "gold.jsonl")}
ec = {r["memory_id"]: r for r in jl(FIX / "insomnia-entity-v2" / "candidates.jsonl")}
er = {r["memory_id"]: r for r in jl(FIX / "insomnia-entity-v2" / "runs" / "sol-low-2026-09-17" / "results.jsonl")}
rows = []
for mid, gold in eg.items():
    base = er[mid]
    norm = lambda spans: sorted((x["field"],x["start_byte"],x["end_byte"],x["text"]) for x in spans)
    exact = norm(base.get("actual_entity_mentions", [])) == norm(base.get("expected_entity_mentions", []))
    rows.append({"id": mid, "gold": gold, "source": ec[mid], "baseline_model":"gpt-5.6-sol",
        "comparison_model":"gpt-6-sol", "reasoning":"low", "baseline_5_6": base, "exact": exact})
mismatch = [r for r in rows if not r["exact"]][:4]
exact_pos = [r for r in rows if r["exact"] and r["gold"].get("expected_entity_mentions")][:4]
zero = [r for r in rows if not r["gold"].get("expected_entity_mentions")][:2]
dumpjl(OUT / "entity_extraction_cases.jsonl", mismatch + exact_pos + zero)

# Entity admission.
ag = jl(FIX / "entity-admission-v3" / "gold.jsonl")
ar = {r["label"]: r for r in jl(FIX / "entity-admission-v3" / "runs" / "sol-low-v3-2026-09-19" / "results.jsonl")}
sel = take_by(ag, lambda r: r["expected_decision"], [
    ("create_new",3), ("reject",3), ("unresolved",1),
])
special = next(r for r in ag if r["label"] == "repo-positive-against-commit")
if special not in sel: sel.append(special)
dumpjl(OUT / "entity_admission_cases.jsonl", [
    {"id": r["label"], "gold": r, "baseline_model":"gpt-5.6-sol",
     "comparison_model":"gpt-6-sol", "reasoning":"low", "baseline_5_6": ar[r["label"]]} for r in sel
])

# Entity disambiguation.
dg = j(FIX / "entity-disambiguation-adversarial-v1" / "gold.json")
dc = jl(FIX / "entity-disambiguation-adversarial-v1" / "candidates.jsonl")
dr = {r["memory_id"]: r for r in jl(FIX / "entity-disambiguation-adversarial-v1" / "run-sol-low-v3" / "results.jsonl")}
drows = [{"id": k, **v} for k,v in dg.items()]
dsel = take_by(drows, lambda r: r["expected_decision"], [
    ("resolve_existing",2), ("create_new",2), ("unresolved",2),
])
out = []
for r in dsel:
    prefix = r["id"].split("-")[0]
    out.append({
        "id": r["id"], "gold": r, "baseline_model":"gpt-5.6-sol",
        "comparison_model":"gpt-6-sol", "reasoning":"low",
        "group": [x for x in dc if x["memory_id"].startswith(prefix + "-")],
        "baseline_5_6": dr[r["id"]],
    })
dumpjl(OUT / "entity_disambiguation_cases.jsonl", out)

# Store resolution: real frozen REL plus empty-store controls.
fg = j(FIX / "entity-store-resolution-frozen-rel-v1" / "gold.json")
fq = {r["memory_id"]: r for r in jl(FIX / "entity-store-resolution-frozen-rel-v1" / "queries.jsonl")}
fe = {r["entity_id"]: r for r in jl(FIX / "entity-store-resolution-frozen-rel-v1" / "entities.jsonl")}
fr = {r["memory_id"]: r for r in jl(FIX / "entity-store-resolution-frozen-rel-v1" / "run-production-v4-sol-low-2026-09-18" / "results.jsonl")}
frows = [{"id": k, **v} for k,v in fg.items()]
fsel = take_by(frows, lambda r: r["expected_decision"], [
    ("resolve_existing",3), ("create_new",2), ("unresolved",1), ("reject",2),
])
resolution = []
for r in fsel:
    base = fr[r["id"]]
    ids = base.get("base_candidate_entity_ids", [])
    resolution.append({"id":"frozen_rel:"+r["id"], "source_id":r["id"], "variant":"frozen_rel",
        "baseline_model":"gpt-5.6-sol", "comparison_model":"gpt-6-sol",
        "reasoning":"low", "gold":r, "query":fq[r["id"]],
        "candidate_entities":[fe[x] for x in ids], "baseline_5_6":base})
zg = j(FIX / "entity-store-resolution-zero-candidate-v1" / "gold.json")
zq = {r["memory_id"]: r for r in jl(FIX / "entity-store-resolution-zero-candidate-v1" / "queries.jsonl")}
zr = {r["memory_id"]: r for r in jl(FIX / "entity-store-resolution-zero-candidate-v1" / "run-sol-low-v4_1b-2026-09-18" / "results.jsonl")}
zrows = [{"id": k, **v} for k,v in zg.items()]
zsel = take_by(zrows, lambda r: r["expected_decision"], [("create_new",1),("unresolved",1),("reject",1)])
for r in zsel:
    resolution.append({"id":"zero_candidate:"+r["id"], "source_id":r["id"], "variant":"zero_candidate",
        "baseline_model":"gpt-5.6-sol", "comparison_model":"gpt-6-sol",
        "reasoning":"low", "gold":r, "query":zq[r["id"]],
        "candidate_entities":[], "baseline_5_6":zr[r["id"]]})
dumpjl(OUT / "entity_resolution_cases.jsonl", resolution)

manifest = {
    "format":"gpt56-vs-gpt6-e2e-v1",
    "purpose":"Small omnibus model-generation comparison corpus assembled from existing calibrated evidence; stages are independent.",
    "baseline_policy":"No new GPT-5.6 inference. Preserve historical 5.6 outputs/judgments and replay only the selected cases with GPT-6.",
    "stages":{
        "insomnia":{"cases":len(insomnia),"contexts":len(episode_ids)},
        "dream":{"cases":len(dream)},
        "chronos":{"cases":len(chronos[:8]),"baseline":"final pre-GPT-6 calibration snapshot"},
        "entity_extraction":{"cases":len(mismatch+exact_pos+zero)},
        "entity_admission":{"cases":len(sel)},
        "entity_disambiguation":{"cases":len(out)},
        "entity_resolution":{"cases":len(resolution)},
    },
    "source_policy":"Every case is copied from an existing calibration/gold/run artifact; no new 5.6 labels are synthesized.",
    "route_map":{
        "insomnia_semantic_ledger":{"gpt5_6":"gpt-5.6-sol/low","gpt6":"gpt-6-sol/low"},
        "insomnia_metadata":{"gpt5_6":"gpt-5.6-luna/low","gpt6":"gpt-6-luna/low"},
        "insomnia_ownership":{"gpt5_6":"gpt-5.6-luna/low","gpt6":"gpt-6-luna/low","via":"insomnia_metadata"},
        "insomnia_wording":{"gpt5_6":"gpt-5.6-sol/low","gpt6":"gpt-6-sol/low"},
        "chronos":{"gpt5_6":"gpt-5.6-sol/low","gpt6":"gpt-6-sol/low","baseline_via":"historical insomnia fallback","comparison_via":"dedicated chronos route"},
        "entity_extraction":{"gpt5_6":"gpt-5.6-sol/low","gpt6":"gpt-6-sol/low"},
        "entity_resolution":{"gpt5_6":"gpt-5.6-sol/low","gpt6":"gpt-6-sol/low"},
        "dream":{"gpt5_6":"gpt-5.6-luna/low","gpt6":"gpt-6-luna/low"},
    },
    "sources":{
        "insomnia":["corpus/insomnia-gold-v3-tuning-v1.json","corpus/insomnia-tuning-v1.jsonl","corpus/insomnia-sol-low-semantic-audit-v1.csv","docs/insomnia-semantic-validation-2026-08-24.md"],
        "dream":["corpus/dream-classifier-regressions-v1.json","corpus/dream-web-audit-v1.json","docs/dream-classifier-v2-design.md"],
        "chronos":["target/chronos-28d-rel-final.jsonl"],
        "entity_extraction":["fixtures/local/calibration/insomnia-entity-v2/gold.jsonl","fixtures/local/calibration/insomnia-entity-v2/candidates.jsonl","fixtures/local/calibration/insomnia-entity-v2/runs/sol-low-2026-09-17/results.jsonl"],
        "entity_admission":["fixtures/local/calibration/entity-admission-v3/gold.jsonl","fixtures/local/calibration/entity-admission-v3/runs/sol-low-v3-2026-09-19/results.jsonl"],
        "entity_disambiguation":["fixtures/local/calibration/entity-disambiguation-adversarial-v1/gold.json","fixtures/local/calibration/entity-disambiguation-adversarial-v1/candidates.jsonl","fixtures/local/calibration/entity-disambiguation-adversarial-v1/run-sol-low-v3/results.jsonl"],
        "entity_resolution":["fixtures/local/calibration/entity-store-resolution-frozen-rel-v1","fixtures/local/calibration/entity-store-resolution-zero-candidate-v1"],
    },
    "notes":[
        "Stage cases are intentionally not a causal chain.",
        "Embedding/vector generation is excluded because it is not a GPT-5.6/GPT-6 generation boundary.",
        "Automatic Entity reconciliation is deterministic and should be evaluated as a shared post-resolution invariant, not as a model-generation score.",
        "Chronos cases preserve the final pre-GPT-6 stage snapshot; score them as stage-regression coverage unless the exact historic route is reconstructed.",
    ],
}
manifest["total_cases"] = sum(x["cases"] for x in manifest["stages"].values())
dumpj(OUT / "manifest.json", manifest)
readme = """# GPT-5.6 vs GPT-6 omnibus v1

Small generation-comparison corpus assembled from existing Reliquary calibration evidence.

The cases are intentionally independent across stages. This is end-to-end coverage, not one
conversation replayed through every stage. GPT-5.6 is the historical control: do not rerun it.
Replay only these selected cases with GPT-6 using the same stage prompt, reasoning level and
deterministic surrounding logic, then compare against preserved 5.6 evidence and gold.

The model mix is part of the fixture contract, not an implementation detail. Main Insomnia uses
Sol-low for the semantic ledger and wording, while the bounded metadata classifier and ownership
classifier use Luna-low. Dream uses Luna-low. Chronos now uses a dedicated explicit Sol-low route;
the preserved GPT-5.6 Chronos snapshot predates that separation and reflects the historical
Insomnia fallback. Entity extraction/admission/resolution use their dedicated Sol-low routes. The
GPT-6 comparison mirrors those model families exactly: Sol->Sol and Luna->Luna.

insomnia_episodes.jsonl contains the exact Episode contexts referenced by insomnia_cases.jsonl.
Those Episodes must be replayed through the full mixed Insomnia chain so Luna metadata is actually
exercised. The remaining files are self-contained stage cases. Chronos is retained as
stage-regression coverage because its snapshot predates GPT-6. Dream's accepted GPT-5.6 Luna
evidence survives as aggregate exact-pair/population measurements; its older per-case Sol output
is retained only as a legacy diagnostic control, not mislabeled as the production Luna baseline.

Automatic Entity reconciliation is intentionally not duplicated here: it is deterministic shared
post-processing and should be checked with the normal reconciliation/invariant suite after each
model's Entity outputs are applied.
"""
(OUT / "README.md").write_text(readme, encoding="utf-8")
print(json.dumps(manifest, indent=2))
