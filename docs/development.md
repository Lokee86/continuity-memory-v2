# Development

Parent index: [Documentation index](INDEX.md)

## Purpose

This document owns repository workflow, layout, verification commands, corpus smoke harnesses, and current measurements.

## Overview

Development uses deterministic local tests plus optional live provider execution. Model-switchboard routing, encrypted credential persistence, auth-header attachment, direct OpenAI-ready embedding/General HTTP transport, ChatGPT/Codex device-code acquisition, and provider-native Codex Responses execution are implemented. Codex OAuth token refresh remains outside the current slice. A separate repo-local CLI exposes current bring-up operations without installation.

## Repository boundary

```text
Cargo.toml / Cargo.lock              core library package + locked dependencies
cli/Cargo.toml / cli/Cargo.lock      detachable repo-local CLI package + lockfile
cli/src/*.rs                         operator command composition over public API
src/config*.rs                      local config framing/object codecs/atomic replacement
src/credential*.rs                 encrypted credentials + authenticated crypto/codecs
src/model_switchboard*.rs           provider capabilities + general/embedding routing
src/model_auth.rs                   credential validation + request auth attachment
src/master_key*.rs                  generated master key + temporary JSON key store
src/cva*.rs                         composition/public CVA lifecycle
src/container*.rs                   physical CVA substrate/global ordering
src/archive*.rs / fragment*.rs      Archive semantics/history/fragments
src/packed_vector*.rs               immutable packed matrices
src/archive_vector*.rs              immutable row -> FragmentId bindings
src/embedding_endpoint.rs           endpoint contract + deterministic simulation
src/compatibility_profile*.rs       tolerant compatibility contracts/probes/reopen
src/vector_generation*.rs           generation publication/history/reopen
src/semantic_search*.rs             exact current-generation semantic retrieval
src/lexical_search.rs / search*.rs  original lexical + hybrid retrieval policy
src/lib.rs                          public exports
examples/archive_roundtrip.rs        prepared Archive corpus smoke
examples/vector_generation_smoke.rs  two-profile vector + retrieval smoke
examples/archive_open_profile.rs     allocator/open-time benchmark

docs/                                current docs, plans, limits, decisions
```

Do not copy large portions of previous Continuity implementations as a migration shortcut. Reuse must preserve current ownership boundaries.

## Verification commands

```text
cargo fmt --check
cargo check
cargo test
cargo fmt --manifest-path cli/Cargo.toml -- --check
cargo check --manifest-path cli/Cargo.toml
cargo test --manifest-path cli/Cargo.toml
python ../engineering-standards/tools/docs_policy/check.py --repo .
```

Prepared Archive round trip:

```text
cargo run --release --example archive_roundtrip -- <graph.jsonl> <output.cva>
```

Two-profile simulated vector-generation smoke:

```text
cargo run --release --example vector_generation_smoke -- <source.cva> <output.cva>
```

The vector smoke copies the source CVA, creates two materially different deterministic simulated endpoints, establishes two compatibility profiles, builds one full Archive generation per profile, syncs, reopens, verifies both current generations, and runs the default hybrid retrieval path through one reopened generation.

Open benchmark:

```text
cargo run --release --example archive_open_profile -- <archive.cva> [runs]
```

The benchmark is standalone and reports median/p90 `Cva::open` time plus allocator-tracked retained and peak additional bytes.

## Prepared-corpus measurements — 2026-08-14

The prepared corpus contains 12 conversations and `1,875` semantic Archive mutations: `1,559` nodes, `35` current branches, `281` fragments, `1,553` content objects, and `3,603` expanded fragment references.

### Current-format baseline

The required-marker CVA with no profiles/matrices/Archive-Vector sets/generations is `2,197,562` bytes.

Release warm-cache `Cva::open`, 50 runs:

```text
median:          25.253 ms
p90:             26.163 ms
retained heap:   757,051 bytes
peak additional: 888,730 bytes
```

The retained-heap increase from the prior slice includes one derived `u64` Archive creation version per fragment, used to verify VectorGeneration source coverage.

### Simulated vector-bearing smoke

The same corpus with two simulated profiles and two current generations is `2,289,410` bytes. Each profile uses 32-dimensional `f32` rows and covers all 281 fragments, for `562` total Archive-Vector rows. The current retrieval smoke reopens that CVA and exercises the original default 30-candidate / top-10 hybrid path through one selected profile/generation without changing file size.

Release warm-cache `Cva::open`, 50 runs:

```text
median:          25.147 ms
p90:             26.145 ms
retained heap:   760,578 bytes
peak additional: 910,413 bytes
```

These 32-dimensional simulated vectors verify machinery and ownership; they are not a performance proxy for eventual production embedding dimensions or scalar representation.

### Live OpenRouter embedding throughput — 2026-08-15

The current configured live route is `qwen/qwen3-embedding-8b` through OpenRouter, with 1024-dimensional L2-normalized output. Throughput was measured over the same 281 durable fragments using a disposable runner under `target/` that calls `OpenAiReadyEmbeddingEndpoint` directly. Cargo compilation, CVA publication, and compatibility probes were excluded from the timed region.

At fixed batch size 16:

```text
concurrency  wall time   fragments/s
1            36.149 s     7.773
4            16.397 s    17.138
8            10.453 s    26.882
16            8.733 s    32.176
```

At fixed concurrency 16:

```text
batch size   wall time   fragments/s
8             9.870 s    28.471
16            8.733 s    32.176
24           11.344 s    24.771
32           11.415 s    24.617
48           17.393 s    16.156
64           14.824 s    18.956
```

The best observed configuration was 16 inputs per request with 16 concurrent requests: `4.14x` the serial throughput and about `1.70x` the throughput of the former 64×16 default on this corpus/provider. The default live batching policy now uses 16×16.

The same live run exposed compatibility-policy calibration rather than an embedding transport failure. Eight repeated profile/re-probe comparisons on the same configured route produced minimum cosine values from `0.99988147` through `0.99993311`; the former policy-v1 threshold `0.99999` rejected every sample. Compatibility policy v2 therefore uses `0.9998`, with the policy version bumped so existing v1 profiles are not silently reinterpreted.

After applying those measured defaults and policy v2, a fresh copy of the zero-vector fixture completed the ordinary `vectors build` path with no batching overrides in `25.251` seconds. The resulting CVA is `3,374,366` bytes and reopens with one 1024-dimensional `f32` matrix containing all `281` fragment rows, one Archive-Vector binding, one compatibility profile, and one active generation at source Archive version `1875` / vector version `1`.

A live default hybrid search over that published generation for `compatibility profiles vector generations` returned the configured top 10 results in `12.828` seconds. All ten carried semantic scores from the active live generation, confirming endpoint re-verification, query embedding, exact semantic scan, row-to-fragment mapping, and hybrid result assembly after reopen.

### Live Codex General transport smoke — 2026-08-15

The configured ChatGPT/Codex route was set to `gpt-5.6-luna` with `low` reasoning and the existing device-code credential. A disposable one-conversation/two-turn CVA was imported and processed through the ordinary `insomnia run` path with one worker. The first backend probe exposed an invalid `reasoning.summary = none` request; the transport was corrected to `auto`. The fresh rerun completed one Episode in one claim with zero retries, zero terminal failures, zero Memories, and no embedding work, confirming live authentication, account routing, Luna model selection, low reasoning, structured Responses output, SSE parsing, and Insomnia provider dispatch.

### Live Insomnia worker concurrency — 2026-08-15

The prepared 12-conversation ChatGPT corpus was converted to the current canonical graph-JSONL development format without adding a production importer. It produced 1,441 canonical text nodes and 66 deterministic import Episodes. Each concurrency point used a fresh copy of the same pre-Insomnia CVA, the configured `gpt-5.6-luna` Codex route at `low` reasoning, and the configured Qwen3 embedding route. Episode-processing time and Memory-Vector time were measured separately so embedding latency did not contaminate the worker scaling curve.

```text
workers   processing     episodes/s   speedup vs 1   retries   terminals
1         456.578 s       0.145         1.00x         0         0
2         240.614 s       0.274         1.90x         0         0
4         108.906 s       0.606         4.19x         0         0
8          79.076 s       0.835         5.77x         0         0
16         46.561 s       1.417         9.81x         0         0
32         29.078 s       2.270        15.70x         0         0
40         28.605 s       2.307        15.96x         0         0
44         30.530 s       2.162        14.95x         0         0
48         21.356 s       3.091        21.38x         0         0
48 repeat  21.162 s       3.119        21.58x         0         0
52         32.516 s       2.030        14.04x         0         0
56         23.527 s       2.805        19.41x         0         0
64         24.579 s       2.685        18.58x         0         0
```

The measured sweet spot was 48 workers for this provider/model/corpus under extractor contract `v2-2`, and the default was changed from 16 to 48 while retaining the 1–64 configurable range. Bracketing runs at 44 and 52 workers took 30.530 s and 32.516 s, while 56 and 64 workers also trailed the repeated 48-worker result. No point produced retries or terminal failures, indicating a latency/contention optimum rather than a hard rate-limit boundary. Contract `v2-2` was subsequently found to contain a shortened semantic extraction prompt rather than the tuned legacy contract; contract `v2-3` restores the legacy policy with only the structural v2 field/evidence mappings. The 48-worker default remains in place provisionally until a focused 32/48/64 recheck confirms that the longer restored prompt does not move the optimum.

The `v2-2` sweep produced 133–157 created Memories and 9–30 rejected candidates from the same 66 Episodes. Every run completed all Episodes, and none requested historical evidence. Those counts must not be treated as evidence about the stability or quality of the legacy extraction policy because the sweep used the shortened `v2-2` prompt. Memory-Vector filling remains a distinct derived phase and is not used in worker-concurrency timing.

Five fresh 48-worker runs under restored contract `v2-3` produced 111, 108, 124, 122, and 116 Memories respectively, with 8–16 rejected candidates, zero retries, zero terminal failures, and zero historical-evidence turns in every run. Audit of the stored Memory bodies found that consolidation and phrasing were generally strong when the same source was selected, but source-selection overlap remained only about 0.60–0.67 pairwise. The audit also exposed cases where vague/question turns were treated as authority without explicit assistant-content provenance, plus residual execution/progress memories and unstable category/type labels.

Extractor contract `v2-4` added a required structured `authority_kind` field with `direct`, `correction`, `adoption`, and `retention` values. Contract `v2-5` added the first deterministic authority-policy layer: direct authority cannot carry assistant content provenance, adoption requires it, bare/callback retention and low-information deictic authority cannot support detailed memories without provenance, unsupported interrogative authority is rejected while explicit asserted tag-questions remain admissible, and obvious prompt/phase/test/commit-style execution receipts are rejected. Contract `v2-6` additionally rejects any candidate that still carries numbered prompt/phase/step progress or checkpoint framing, even when the candidate also contains useful project state; the extractor must emit the durable state without the numbered progress reference. Contract `v2-7` splits contextual provenance into assistant-authored **authority sources** versus **grounding sources** used only to resolve referents in user-owned statements/corrections. The existing persisted `content_source_*` fields now mean authority provenance only; new persisted `grounding_source_*` fields carry referent grounding. These checks remain deliberately narrow and do not claim general semantic entailment.

### Insomnia semantic gold set — 2026-08-15

`corpus/insomnia-gold-v1.json` is retained as the historical 40-case contract used for the recorded v2-3/v2-5/v2-6 comparisons below. `corpus/insomnia-gold-v2.json` is the current 40-case contract: the same 32 retain / 8 omit source anchors, but with separate `authority_source` and `grounding_source` policy. Four retained cases explicitly require grounding without assistant authority: creatureServer mothballing, the BrowserOS existing-MCP correction, ShipStats-now, and the deleted-assets correction. ShipStats is corrected from adoption to user correction and its target is narrowed to `Add ShipStats now.`; assistant context only resolves `it` and cannot authorize later scope details.

`tools/evaluate_insomnia_gold.py` now scores gold v2 mechanically for selection, assistant-authority provenance, grounding provenance, acceptable metadata, explicit forbidden-content checks, and receipt cleanup. Semantic equivalence to each case's `semantic_target` remains a human/model judgment rather than lexical overlap. Historical percentages below remain gold-v1 measurements and are not silently recomputed under v2.

Retrospective scoring of the existing five-run audits gives:

```text
contract   selection   provenance   metadata   content/receipt guards
v2-3         64.5%       95.5%       82.1%             92.9%
v2-5         67.0%       98.0%       92.2%             96.1%
```

The `v2-5` per-run selection scores were 57.5%, 70.0%, 67.5%, 62.5%, and 77.5%. Selection remains the dominant measured failure mode. The high provenance percentage is conditional on a gold-retain source actually being selected; it does not mean semantic support is generally validated. Gold v1 treated `ok, so we do want to add it now` as adoption; the later account/corpus audit showed that judgment was wrong and gold v2 corrects it to user authority plus referent grounding.

A controlled `v2-6` comparison used the same clean zero-Memory base CVA, 66 Episodes, 48 workers, embedding route, batching, and contract for three fresh Luna-low runs and three Luna-medium runs. Reasoning effort was the intended variable, and the configured route was restored to low afterward.

```text
reasoning   wall times (s)          avg wall   Memories       rejected       evidence turns   selection   provenance   metadata   guards   source Jaccard   strict semantic target
low         37.766 49.234 39.373     42.12     102 110 110     16 10 13       22 0 8           63.3%       98.2%        87.5%      100%     0.536            60.0%
medium      73.377 74.949 65.444     71.26     129 132 126     17 15 22       36 8 22          65.0%       98.4%        83.6%      100%     0.656            60.0%
```

`source Jaccard` is the mean pairwise overlap of all source nodes selected within the three runs at that reasoning level. `strict semantic target` is an explicit manual/model review against the 40 tracked semantic targets, counting omissions, incomplete targets, unsupported authority/provenance, and false-positive Memories as failures; it is intentionally stricter than the mechanical source-selection metric.

Medium reasoning made source selection more repeatable and increased raw Memory production, but did not improve aggregate semantic-target agreement. It gained only 1.7 mechanical selection points, reduced acceptable metadata agreement, took about 69% longer wall-clock time, and produced about 20% more Memories. Stable failures were largely unchanged: future ship variants, player-color identity, the narrow current custom-room-code fact, narrow server-authoritative collision state, the contextual deleted-assets correction, and calibrated prompt sizing were missed in all six runs, while the Godot-first question was incorrectly converted into a decision in all six. ShipStats was selected once at each reasoning level but still lacked the required assistant-content provenance. Medium also introduced one Prompt-41 receipt-derived Memory and one `fix it?` execution-request Memory that low omitted correctly.

The result does not support spending more reasoning budget on Insomnia extraction. Luna-low remains the selected bring-up default. The remaining quality problem is semantic selection/support policy: additional reasoning stabilizes a broader choice of sources, but does not make that choice materially more correct. Contract `v2-6` did raise the tracked explicit forbidden-content/numbered-receipt guard score to 100% in all six runs, but general semantic authority, entailment, and non-numbered execution-state handling still require a stronger validation/review mechanism rather than more lexical heuristics. The comparison is only three runs per level, so small numeric differences should not be treated as precise population estimates.

### Gold-set account-context audit — 2026-08-15

`corpus/insomnia-gold-account-context-audit-v1.json` audits all 40 gold cases against the account-level context retrieval surface available to the assistant and against the original local conversation neighborhoods in the prepared corpus. This is not a dump of platform-internal memory objects; it distinguishes surfaced user facts/preferences/constraints from recoverable historical conversation/file context.

Of the 32 retained cases, 19 have strong independently recoverable account support, 9 have only partial/related account support, and 4 have no independently recovered copy of the exact target. Those counts do **not** imply that account-wide retrieval should participate in authority. Several strong external matches contain broader or later state that would incorrectly expand the current user's proposition if treated as semantic authority.

The original audit focused on the 9 gold-v1 cases marked as requiring assistant provenance and found nearby conversation context for all of them. Re-review under the provenance split identified four cases where context is **grounding rather than authority**: creatureServer mothballing, BrowserOS existing-MCP, ShipStats-now, and deleted assets. The corpus still does not demonstrate a need for vector search as the first semantic-support mechanism. Deterministic current-Episode/nearby ancestry should be exhausted first; vector retrieval remains a fallback for genuinely distant unresolved references.

Contract `v2-7` implements that correction. Candidate structured output now has separate `authority_source_*` and `grounding_source_*` tuples. Authority sources must be earlier assistant turns explicitly adopted/retained by the user. Grounding sources may be earlier user or assistant turns but can only identify a referent; they cannot authorize semantic details absent from the user source turn. Both source kinds must precede the user authority and must be inside the authoritative Episode or returned by the bounded evidence round. Memory record format is deliberately bumped from `CVAMEMF1/CVAMEMR1` to `CVAMEMF2/CVAMEMR2` to persist grounding node identity separately; no migration shim is added during the rebuild.

The negative cases reinforce the same rule. Related history exists for several rejected questions/requests, but retrieval must not convert `so... fix it?`, the local-spawner question, or the Godot-first question into durable decisions. Likewise, account-wide collision and custom-room-code context can contain broader/later project state; similarity alone must never make that state authoritative for the current turn.

### Benchmark baseline — 2026-08-15

This repository state is the benchmark baseline for subsequent retrieval-quality changes. The baseline restores the retrieval behavior used by the existing Long Memory Evaluation machinery: 8-turn / 2-overlap fragments, `qwen/qwen3-embedding-8b` at 1024 dimensions, exact cosine semantic retrieval, the original lexical scoring formula, 30-candidate lexical/semantic fusion with `0.45/0.55` weights, duplicate-range removal, overlap diversification, and top-10 output.

The historical LME score has not been rerun against this v2 implementation. The baseline claim is therefore that the benchmark-required machinery and retrieval semantics are restored and live-tested, not that v2 has independently reproduced the prior judged score. Changes after this point that can affect retrieval quality should be compared against this baseline explicitly.

A current-format cold-cache sample has not yet been recorded. Larger-population realistic-dimension vector runs require repeated cache eviction plus one open per sample before conclusions about cold scaling, segmentation, or checkpoints.

## Failure modes

- `cargo fmt --check` catches Rust formatting drift.
- `cargo check` catches type/compile errors.
- Core `cargo test` owns focused library behavior, including config replacement, encrypted credential round trips/tamper rejection, switchboard auth binding, and master-key generation/reload/redaction.
- The separate CLI fmt/check/test commands verify that the detachable package compiles only against the public library surface.
- Command-level CLI smoke tests exercise CVA create/info/verify, graph import/archive inspection, simulated profile/vector/search execution, and config show/verify without installing the binary. Live `vectors probe/build` requires an external endpoint and credential and is not part of deterministic CI.
- The documentation checker validates repository documentation policy but not semantic correctness.
- `archive_roundtrip` fails on Archive reconstruction/count/content disagreement.
- `vector_generation_smoke` fails if profile separation, full-fragment generation building, current-generation reconstruction, row counts, or reopened default hybrid retrieval disagree.

## Related docs

- [Architecture](architecture.md)
- [Maintainer map](maintainer-map.md)
- [Documentation coverage](documentation-coverage.md)
- [Behavioral contracts](behavioral-contracts.md)
- [Repo-local CLI](cli.md)
- [ADR 0007](decisions/0007-compatibility-profiles-and-vector-generations.md)
- [ADR 0009](decisions/0009-expandable-model-switchboard.md)
- [ADR 0010](decisions/0010-encrypted-credential-objects.md)
- [ADR 0011](decisions/0011-detachable-repo-local-cli.md)

## Notes

Additional provider-native/local adapters, Codex OAuth token refresh, and production import interfaces are not implemented in this repository.