# GPT-5.6 vs GPT-6 omnibus v1

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
