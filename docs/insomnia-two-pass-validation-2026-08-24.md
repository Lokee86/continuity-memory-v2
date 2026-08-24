# Insomnia two-pass validation — 2026-08-24

Parent index: [Documentation index](INDEX.md)

## Purpose

This record captures the small-fixture validation of a two-pass Insomnia semantic architecture, the pass-1 tuning sequence, and the model/provider comparison against `gpt-5.6-sol`, `stealth/ox-alpha`, and `gpt-5.6-luna` at low reasoning.

The authoritative Insomnia runtime now implements this architecture under extractor contract `v3-0`. The implementations in `examples/` and `tools/` remain benchmark/provider-compatibility harnesses rather than production execution paths.

## Overview

The focused fixture supports the two-pass decomposition and establishes `gpt-5.6-sol` at low reasoning as the current selector candidate. Sol-low reached complete durable-state coverage and omit cleanliness after pass-1 tuning; Ox Alpha matched recall but lost precision/provenance and required provider-specific structural handling; Luna-low fell below the observed selector capability floor. A controlled pass-2 ownership test then removed the remaining synthesis-side authority leakage, and that ownership boundary is now implemented in authoritative Insomnia `v3-0`.

## Fixture and evaluator

Routine tuning uses:

- `corpus/insomnia-tuning-v1.jsonl`: 11 isolated Episodes / 314 turns.
- `corpus/insomnia-gold-v3-tuning-v1.json`: 19 cases, comprising 15 retained-state cases and 4 omit cases.
- `tools/evaluate_insomnia_gold.py`: reports anchor fidelity, durable state coverage, omit cleanliness, assistant-authority provenance, grounding provenance, metadata, and content/receipt guards.

Semantic-target equivalence remains a manual/model judgment; the percentages below are the mechanical gold-v3 tuning metrics.

## Architecture under test

The experiment decomposes extraction into two model calls:

1. **Authority/disposition ledger:** account for every user turn at clause level and decide `retain`, `omit`, or `superseded`; assign `direct`, `correction`, `adoption`, or `retention`; preserve lifecycle/modality; distinguish assistant authority from referent grounding.
2. **Memory synthesis:** write durable Memory prose only from ledger entries already marked `retain`.

The decomposition is intended to remove the single-pass requirement that one generation simultaneously discover authority, decide disposition, resolve lifecycle/provenance, and synthesize final Memory prose.

Pass 1 remains the semantic gate. Pass 2 is not allowed to resurrect omitted or superseded state.

## Deterministic infrastructure ownership

Several fields proved inappropriate to leave to probabilistic byte reproduction or unconstrained identifier generation:

- Dynamic per-Episode schemas constrain user source IDs, assistant authority IDs, and grounding IDs to real Episode turns.
- Continuity owns byte-exact provenance. If a model selects the correct source ID but slightly changes Markdown/punctuation in a quote, the harness repairs the quote from the authoritative source turn.
- Direct/correction authority cannot acquire assistant authority provenance merely because synthesis emits one.
- Self-grounding is stripped.
- Experimental resume support reuses completed ledger/synthesis artifacts instead of rerunning already-completed inference.

The Ox/Nous compatibility harness additionally requires every user-turn ID as a tool-schema key because Ox repeatedly skipped pure-question turns when completeness was expressed only in prompt text.

## Pass-1 tuning sequence with Sol-low

The initial full 11-Episode two-pass Sol run produced 94 Memories and scored:

| Metric | Initial two-pass |
| --- | ---: |
| Anchor fidelity | 84.2% |
| State coverage | 100.0% |
| Omit cleanliness | 50.0% |
| Authority | 79.3% |
| Grounding | 62.1% |
| Metadata | 75.0% |
| Guards | 96.6% |

The important positive result was immediate `100%` durable-state coverage. The dominant problem was over-permissive disposition/provenance rather than failure to discover state.

Pass 1 was then tuned specifically around measured failures:

- pure numbered execution/checkpoint receipts are omitted;
- recalled plans, challenges, open questions, and alternatives do not become decisions;
- a tag question may retain an independently asserted requirement (`X has to be Y, right?`);
- mixed checkpoint/status turns are split so durable status survives while numbered progress is stripped;
- terse assent to one clear assistant proposal may be a durable adoption;
- authority provenance and grounding provenance are separate roles;
- direct/correction state does not use assistant authority;
- grounding is used only to resolve a genuinely unclear referent;
- older overlapping state is superseded when later user authority actually replaces/refines it.

An intermediate strict run (`v4`) reached 100% anchor fidelity, state coverage, omit cleanliness, grounding, and guards, but over-pruned part of the mixed Prompt-72 status. A broader classification experiment (`v5`) regressed and was rejected. The retained `v6` contract restored the mixed status while preserving the important selection behavior.

## Sol-low v6 result

The tested Sol-low v6 run produced 54 Memories:

| Metric | Sol-low v6 |
| --- | ---: |
| Anchor fidelity | 94.7% |
| State coverage | 100.0% |
| Omit cleanliness | 100.0% |
| Authority | 95.8% |
| Grounding | 100.0% |
| Metadata | 60.0% |
| Guards | 100.0% |

The Prompt-72 turn is semantically handled as intended: `everything seems to be working OK so far` and the remaining visual bugs/oddities are retained, while the `prompt 72 completed` checkpoint framing is removed.

The remaining anchor loss is ShipStats: the earlier correction/decision source was treated as superseded by a later valid direct restatement. Durable state coverage therefore remains 100% even though exact anchor fidelity is 94.7%.

The remaining authority noise exposed a second architectural issue: the original experimental pass 2 could choose provenance/classification independently from pass 1. In observed cases, a correct direct ledger entry was synthesized with an invented assistant authority source. That synthesis ownership leak motivated the fixed-group `v3-0` production boundary described below.

## Ox Alpha through Nous

Model: `stealth/ox-alpha`, low reasoning.

### Transport findings

Direct Nous endpoint:

`https://inference-api.nousresearch.com/v1/chat/completions`

Normal `response_format` / JSON-schema mode is not reliable for this model/provider combination. Ox repeatedly returned coherent Markdown despite a strict JSON-schema request. Earlier requests also encountered upstream capacity/timeout behavior.

Forced tool/function calling is usable. The experimental client now supports streamed `tool_calls` arguments and retries. Additional structural safeguards were necessary because Nous did not consistently enforce all declared required/enumerated fields:

- required per-user-turn tool keys for ledger completeness;
- deterministic quote canonicalization;
- deterministic repair of invalid structural authority values when the semantic/provenance state uniquely determines the valid value;
- inheritance of missing synthesis category/type/authority fields from the matching retained ledger only when unambiguous;
- adoption authority can inherit the immediately preceding assistant parent when the model selected adoption but omitted the source ID;
- resume support preserves completed Episode artifacts.

A one-Episode end-to-end probe succeeded with 11 Memories in 90.545 seconds. The full 11-Episode run completed with 62 Memories.

### Ox/Nous score

| Metric | Ox Alpha / Nous |
| --- | ---: |
| Anchor fidelity | 94.7% |
| State coverage | 100.0% |
| Omit cleanliness | 75.0% |
| Authority | 90.9% |
| Grounding | 86.4% |
| Metadata | 62.5% |
| Guards | 95.5% |

Ox is competitive on recall but more permissive than tuned Sol. Two clear pass-1 semantic errors remained:

- `prompt-41-receipt`: Ox explicitly retained `completed through Prompt 41` as durable project progress.
- `prompt-72-mixed-status`: Ox retained the durable status but left `prompt-72 milestone` framing in the proposition.

ShipStats was also classified as adoption with assistant authority rather than user correction plus grounding.

The creatureServer authority/grounding mechanical failure understates the semantic output: Ox emitted two Memories from the same mixed user turn, including a correct direct `creatureServer is mothballed` Memory with correct grounding, but the evaluator selected the sibling surface-review adoption as the anchor match.

## Luna-low comparison

The same v6 semantic contract was run with `gpt-5.6-luna` at low reasoning and four workers.

The raw Luna run omitted six user turns from two ledgers. None of those turns were gold anchors; they were a pasted Prompt 64K, pure questions, transient cleanup/tooling complaints, and pasted terminal output. For completion of the model comparison only, those six missing turns were deterministically inserted as explicit `omit` ledger entries under the existing pass-1 rules. This repair did not supply any missing gold state.

The completed Luna run produced 48 Memories:

| Metric | Luna-low |
| --- | ---: |
| Anchor fidelity | 68.4% |
| State coverage | 73.3% |
| Omit cleanliness | 50.0% |
| Authority | 66.7% |
| Grounding | 86.7% |
| Metadata | 72.7% |
| Guards | 93.3% |

Luna genuinely failed to select four durable gold states:

- future ship variants;
- current custom-room-code state;
- server-authoritative collision constraint;
- deleted-assets contextual correction.

It also retained Prompt-41 progress and phase-renumbering. The result establishes a meaningful model-capability floor for the pass-1 semantic task; the two-pass decomposition alone does not make a weaker selector reliable.

## Current model ranking

On this 11-Episode tuning fixture and current contract:

`Sol-low > Ox Alpha-low >> Luna-low`

Comparison:

| Metric | Sol-low v6 | Ox Alpha / Nous | Luna-low |
| --- | ---: | ---: | ---: |
| Memories | 54 | 62 | 48 |
| Anchor fidelity | 94.7% | 94.7% | 68.4% |
| State coverage | 100.0% | 100.0% | 73.3% |
| Omit cleanliness | 100.0% | 75.0% | 50.0% |
| Authority | 95.8% | 90.9% | 66.7% |
| Grounding | 100.0% | 86.4% | 86.7% |
| Metadata | 60.0% | 62.5% | 72.7% |
| Guards | 100.0% | 95.5% | 93.3% |

Sol-low remains the preferred Insomnia selector for the next implementation step. Ox Alpha is worth retaining as a compatibility/model benchmark, particularly because its state coverage matches Sol, but it currently needs more transport/structure normalization and loses on disposition/provenance discipline. Luna-low is below the observed pass-1 capability threshold.

## Pass-2 ownership validation and production integration

The controlled `v7` comparison reused the exact Sol-low v6 ledgers and changed only pass 2. Sixty-eight retained ledger clauses collapsed deterministically into 49 structurally compatible synthesis groups. The wording model received only required group keys and `{title, content}` output fields; provenance, classification, lifecycle, source IDs, and group membership were no longer model-editable.

The controlled result produced 49 Memories with `94.7%` anchor fidelity, `100%` state coverage, `100%` omit cleanliness, **`100%` authority**, `100%` grounding, `60%` metadata, and `100%` guards. Compared with v6, authority rose from `95.8%` to `100%` with no loss in coverage, omit cleanliness, grounding, or guards. The unchanged metadata score confirms those residual classifications belong to pass 1 rather than synthesis.

A fresh end-to-end `v7` Sol-low run produced 51 Memories and scored `89.5%` anchor fidelity, `93.3%` state coverage, `100%` omit cleanliness, `100%` authority, `100%` grounding, `71.4%` metadata, and `100%` guards. Its substantive coverage miss was `future-ship-variants`, which pass 1 stochastically omitted; no synthesis-side authority/provenance failure reappeared.

Authoritative Insomnia now implements this ownership boundary as extractor contract `v3-0`:

- **Pass 1 owns:** disposition, authority kind, lifecycle, source identity, assistant-authority identity, grounding identity, category, and type.
- Retained clauses are deterministically grouped only when all those structural fields match.
- **Pass 2 owns:** title/body wording for every required group and cannot add, drop, merge, split, or reclassify groups through its schema.
- Continuity reconstructs exact source/authority/grounding provenance from selected IDs rather than asking the wording model to reproduce source bytes.
- One bounded Archive evidence round remains inside pass 1; the final ledger is resolved before synthesis begins.
- Candidate identity includes deterministic ledger semantic material so structurally distinct Memories from one source turn do not collide when exact full-turn provenance is reused.

The evidence still does not justify a third general semantic review pass. A future third pass, if measured failures justify one, should be a narrow adversarial reject/flag validator rather than another state-discovery or rewriting pass.

The next milestone is one full 66-Episode gold-v3 confirmation against authoritative `v3-0`, followed by concurrency recalibration for the selected Sol/two-pass model mix. The historical 48-worker Luna optimum must not be assumed valid.

## Experimental implementation

- `examples/insomnia_two_pass_sol/main.rs`
- `examples/insomnia_two_pass_sol/contract.rs`
- `tools/insomnia_two_pass_prompts.py`
- `tools/nous_json_client.py`
- `tools/run_insomnia_two_pass.py`
- `examples/run_two_pass_openrouter.rs`

The Rust harness uses the configured Insomnia route and forces low reasoning in memory without saving config. The Python prompt module imports the Rust contract text so Ox/Nous tests use the same semantic prompt. `target/insomnia-*` outputs are disposable local benchmark artifacts and are not repository inputs.

## Related docs

- [Development](development.md)
- [Roadmap](roadmap.md)
- [Architecture](architecture.md)
- [Current limitations](current-limitations.md)
- [ADR 0012 — deterministic Episodes and Insomnia Memory authority](decisions/0012-deterministic-episodes-and-insomnia-memory-authority.md)

## Notes

These measurements are specific to the 11-Episode tuning fixture and current prompts/provider behavior. They establish and now protect an implemented architecture, not population-level model accuracy. Full 66-Episode gold-v3 confirmation remains the next semantic milestone for authoritative `v3-0`.
