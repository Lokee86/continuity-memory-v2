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

`corpus/insomnia-gold-v1.json` is retained as the historical 40-case contract used for the recorded v2-3/v2-5/v2-6 comparisons below. `corpus/insomnia-gold-v2.json` is retained as the provenance-split 40-case contract introduced with extractor v2-7. `corpus/insomnia-gold-v3.json` is the current contract. It keeps 32 durable-state cases and 8 omit cases, but separates the original benchmark anchor from the user-authoritative source or sources allowed to satisfy current semantic-state coverage. Each case now records `anchor_disposition`, `state_retain`, and `coverage_sources`; alternate sources may override provenance/metadata/guard expectations when their authority form differs from the original anchor.

Gold v3 corrects the stale player-color target: the earlier `modulate` decision is marked `superseded`, and current-state coverage comes from the later user-authoritative shader-hue decision in the same Episode. Packet migration and ShipStats also enumerate later valid user-authoritative sources that may satisfy state coverage without pretending they are the original benchmark anchor. The four v2 grounding distinctions remain intact: creatureServer mothballing, BrowserOS existing-MCP, ShipStats-now, and deleted assets use grounding only to resolve referents and never to authorize additional semantic detail.

`tools/evaluate_insomnia_gold.py` now defaults to gold v3 and reports exact **anchor fidelity**, durable **state coverage**, **omit cleanliness**, assistant-authority provenance, grounding provenance, metadata, and content/receipt guards separately. Coverage is source-based and may use only explicitly enumerated valid authority nodes; where one user turn emits multiple unrelated Memories, a narrow coverage selector prevents the sibling Memory from satisfying the wrong case. Provenance or metadata failure does not erase state coverage: for example, a BrowserOS Memory may cover the intended state while still failing required grounding. Semantic equivalence to each case's `semantic_target` remains a human/model judgment rather than lexical overlap. Historical percentages below remain gold-v1 measurements and are not silently recomputed under v2 or v3.

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

### Candidate-level semantic-review experiment — 2026-08-16

A disposable second-pass semantic reviewer was implemented and live-tested, then removed rather than retained in the runtime. The reviewer received only each extracted candidate plus its authoritative user turn and permitted authority/grounding source. An initial `supported` / `narrow` / `unsupported` contract over-narrowed 57 of 113 reviewed candidates, so a second contract separated `authority_valid`, `durable`, and `full` / `partial` / `none` semantic support and prohibited stylistic compression.

The second contract was compared over three fresh Luna-low / 48-worker runs against three fresh v2-7 controls on the same current-format 66-Episode fixture:

```text
mode       Memories/run       avg Memories   retain-source recall   omit-source clean   avg mechanical selection
control    103 / 97 / 97         99.0             65.6%                79.2%                  68.3%
review     86 / 74 / 79          79.7             57.3%                87.5%                  63.3%
```

The reviewer improved precision on the tracked omit cases: the Godot-first interrogative false decision disappeared in all review runs and the Prompt-41 receipt false positive seen in one control run also disappeared. However, `phase-renumbering` remained a false positive in all three control and all three review runs. More importantly, the blanket review removed too much useful state: retain-source recall fell by 8.3 points and average mechanical gold selection fell by 5.0 points.

The provenance breakdown rules out using the same reviewer only because contextual provenance exists. Gold cases requiring assistant authority were selected at 81.0% in both control and review runs. Grounding-required cases fell from 33.3% to 25.0%, and ordinary retained cases fell from 66.7% to 55.6%. A candidate-only reviewer also cannot recover a durable source turn the extractor never selected.

A direct Episode-boundary inspection confirmed that the required grounding material for all four gold-v2 grounding cases is already present inside the same authoritative Episode: creatureServer, BrowserOS, ShipStats, and deleted assets do not require broader ancestry or vector retrieval to resolve their tracked referents. The remaining measured problem is therefore **initial semantic source selection inside an already-sufficient Episode**, followed by faithful synthesis from selected authority—not missing retrieval context.

The experimental reviewer code and calibration-only CLI hooks were removed after measurement. The next quality experiment should separate **per-user-turn retain/omit source selection** from Memory synthesis so every potential authority turn receives an explicit disposition before consolidation. Broader retrieval, including vectors, remains a fallback for future cases that actually lack sufficient Episode-local grounding.

### Sol-low semantic-review follow-up — 2026-08-16

The same v2-7 66-Episode / 48-worker fixture and second-pass reviewer contract were then retested with `gpt-5.6-sol` at low reasoning in a detached disposable worktree. Three fresh Sol controls were compared with three fresh Sol runs using the reviewer; the main runtime and configured Luna route were not changed.

```text
mode       Memories/run        avg Memories   retain-source recall   omit-source clean   avg mechanical selection   source Jaccard
control    154 / 148 / 158        153.3             85.4%                62.5%                  80.8%               0.695
review     122 / 121 / 126        123.0             71.9%                87.5%                  75.0%               0.638
```

Sol materially changed the first-pass extraction result. Compared with the earlier Luna-low v2-7 controls, Sol controls raised retain-source recall from `65.6%` to `85.4%` and average mechanical selection from `68.3%` to `80.8%`, while producing substantially more raw Memories. A direct body audit of the first Sol pair also recovered several targets that were stable Luna misses: future ship variants, the narrow custom-room-code current fact, deleted-assets grounding, and calibrated prompt sizing. Sol still consistently missed player-color identity, the narrow collision-authority target, and the mixed Prompt-72 state in the observed runs.

The blanket reviewer still did not justify itself. It removed the Godot-first false decision and Prompt-41 receipt in all three review runs and improved omit cleanliness from `62.5%` to `87.5%`, but it reduced retain recall by 13.5 points, lowered average mechanical selection by 5.8 points, and made source selection less repeatable. It also still admitted phase-renumbering in two runs and one finish-job request in one run. The three Sol controls took `50.846`, `71.347`, and `63.431` seconds end to end; review runs took `73.842`, `88.115`, and `66.900` seconds, about 23% slower on average.

The useful conclusion is therefore **model choice before architectural expansion**. Sol-low is now the stronger extractor candidate and should receive a full strict semantic-target review before replacing Luna-low as the configured default. If a second pass is retained at all, the next experiment should gate semantic review only to structurally ambiguous/high-risk authority candidates rather than reviewing every Memory. This may preserve Sol's high recall while filtering question/request/receipt false positives. No result here establishes a need for vector retrieval.

### Strict Sol-low semantic-target audit — 2026-08-16

The three-run Sol-low comparison was regenerated in a detached audit worktree so the actual Memory bodies could be judged rather than relying on source-node selection alone. The fresh controls created `147`, `150`, and `149` Memories from the same 66-Episode v2-7 fixture, with zero retries or terminal failures. Exact-anchor mechanical selection was `72.5%`, `75.0%`, and `85.0%` (77.5% mean); the difference from the earlier 80.8% three-run sample confirms that source selection remains stochastic even with Sol.

`corpus/insomnia-sol-low-semantic-audit-v1.csv` records the 40-case body-level audit. A run counts as semantically acceptable there only when the target is faithfully retained, correctly omitted, or legitimately represented by a different/later user-authoritative Memory; incomplete targets, unsupported expansion, provenance misuse, and false durable state fail. Under that stricter rule the three runs were `30/40`, `29/40`, and `32/40` acceptable (`75.8%` mean). This is not directly comparable to the historical v2-6 60% strict score because the current audit uses corrected gold-v2 semantics and additionally recognizes valid alternate/superseding authority.

The audit also exposed an evaluator/gold distinction that the exact-anchor metric cannot represent. Packet migration and ShipStats were mechanically missing in some runs while the same durable state was retained from another valid later user-authoritative turn. More importantly, the player-color gold target freezes the earlier `modulate` implementation even though a later user turn in the same Episode explicitly supersedes it with shader hue values while preserving the durable local-default/remote-distinct identity rule. R1/R2 correctly preferred that later state. Future gold should therefore separate **authority-anchor fidelity** from **current semantic-state coverage** and allow explicit supersession/alternate-authority relationships rather than treating every non-gold anchor as semantic loss.

The remaining stable failures are much more diagnostic than the headline score:

- `adopt-c-family-adapter` is missed in all three runs despite an explicit `Fuckit, add that` immediately after the shared C/C++ adapter proposal. This is a true adoption-selection failure, not missing context.
- `collision-authority-narrow` is missed in all three runs. The user asserts that collision has to stay server-authoritative and adds a confirmation tag; the extractor's strong anti-question instruction is suppressing an asserted proposition before the deterministic tag-question policy can see it.
- `prompt-41-receipt` is falsely retained in all three runs. `completed through 41` is being combined with the contextual contents of Prompt 41 to manufacture an implementation fact. The current receipt wording permits retaining a resulting state after completion and is therefore too permissive about *implied* results.
- `prompt-72-mixed-status` is missed in all three runs. The same anti-receipt policy suppresses the whole turn instead of discarding only the numbered checkpoint clause and retaining the independently asserted current state (`generally working`, with visual bugs/oddities remaining).
- Provenance remains semantically leaky in a smaller set of cases: BrowserOS can be named without grounding, and prompt-sizing Memories sometimes use assistant context to add packet-migration-specific framing that the user's correction did not need.

These failures point to task decomposition rather than retrieval. The necessary evidence is already Episode-local, and the 64-candidate ceiling is not binding. The current single pass asks one model simultaneously to select durable user propositions, classify authority, distinguish questions/receipts, resolve referents, preserve lifecycle/modality, consolidate related claims, write standalone Memories, and attach provenance. Sol handles that bundle better than Luna, but the stable boundary failures remain.

The next quality experiment should therefore replace free-form `Episode -> Memories` selection with an explicit **clause-level authority/disposition ledger** before Memory synthesis. Every user turn should receive an explicit disposition; mixed turns may produce both omitted and retained clauses. The selector should record exact source span, retain/omit, authority kind, lifecycle/modality, and whether assistant authority or referent grounding is required. Deterministic validation can then require complete turn accounting and prevent a receipt from authorizing the contents of a completed task. A second synthesis step should consolidate only already-authorized propositions while preserving modality and provenance. Sol-low is the current best candidate for the hard selection pass; a smaller model can be tested for synthesis afterward because synthesis quality has generally been strong once the correct source is selected.

Gold/evaluator v3 now implements that correction. On the same three preserved Sol-low controls, exact anchor fidelity is `75.0%`, `77.5%`, and `87.5%` (`80.0%` aggregate), while explicitly allowed state-source coverage is `78.1%`, `81.2%`, and `84.4%` (`81.2%` aggregate). Omit cleanliness is `87.5%` in all three because Prompt 41 remains the one stable false-positive omit case. The split now makes packet-migration/ShipStats alternate authority and the superseding player-color hue decision visible without erasing provenance defects such as BrowserOS R2's missing grounding. These are mechanical source/contract metrics only; strict semantic-target equivalence remains separately judged. No result in this audit supports adding vector retrieval; vectors cannot fix propositions that are already present in the Episode but misclassified or skipped.

### Insomnia routine tuning corpus — 2026-08-16

Routine semantic tuning no longer uses the full 12-conversation / 66-Episode fixture. Gold v3 spans 28 distinct source Episodes, which is still too expensive for repeated frontier-model iteration. `corpus/insomnia-tuning-v1.jsonl` is the deliberately adversarial core fixture: 11 isolated Episodes, 314 turns, and 19 gold-v3 cases (`15` retained-state cases and `4` omit cases). `corpus/insomnia-gold-v3-tuning-v1.json` is the matching evaluator subset. Source-node IDs and source content are preserved verbatim; fixture-local conversation/Episode IDs isolate each selected path cleanly.

The selected cases cover the failure modes that have actually driven Insomnia changes: short/deictic adoption, correction plus grounding, unsupported contextual elaboration, asserted tag questions versus pure questions, pure execution receipts, mixed receipt/current-state turns, transient phase metadata, future modality, supersession, over-atomization, long governing architecture with trailing implementation commentary, and simple direct-state sanity. One redundant C-family precision-adoption case was intentionally dropped because equivalent adoption behavior is already exercised by two C-family cases plus the separate `alright, that works` case.

For a two-pass extractor this reduces the nominal no-evidence model-call floor from `66 * 2 = 132` calls to `11 * 2 = 22`, about an `83%` reduction. Use the 11-Episode fixture for routine prompt/contract/model tuning. Use full gold v3 / the 66-Episode fixture only for milestone confirmation after the small corpus is stable, not for every iteration.

### Atomic self-cleaning Insomnia completion — 2026-08-16

Successful Episode processing no longer publishes Memory revisions one at a time followed by a separate attempt/work update. The processor first validates extraction, stages any new content-addressed Memory bodies plus their CVA-global version tickets, and then writes one `CVAINSC1` completion chunk containing every newly visible Memory record together with the compact successful Episode receipt. The same chunk is consumed by both Memory and Insomnia rebuild state. Until that chunk is fully present, staged bodies/tickets are inert and no new Memory is current.

Container reopen now treats an incomplete final length-prefixed chunk as an interrupted append: it truncates the file back to the start of that trailing chunk and continues from the last complete boundary. A deterministic synthetic-endpoint fault test processes an Episode, truncates the CVA halfway through its completion chunk, reopens it, verifies zero new Memories and a reclaimed Pending Episode, then retries and verifies exactly one clean final publication. Zero-Memory, multi-Memory, reopen, and retry-before-success cases are also covered.

Operational retention is deliberately narrow. Retryable failures and terminal failures persist only current `InsomniaWork` state; they no longer append a separate durable attempt-history record. On successful processing, any prior logical attempt history for that Episode is collapsed to one final completion receipt containing the producing model/contract, timestamps, Memory IDs, and rejected count. Detailed extraction responses, evidence bundles, disposition/synthesis intermediates, and retry traces are not made durable by this path.

This is **logical operational vacuum**, not general physical CVA compaction. Earlier superseded Work chunks and orphaned staging bodies/version tickets can remain as unreachable append-only bytes until the general CVA packing/vacuum layer exists. Rewriting the whole CVA on every Episode completion would be substantially more pathological than retaining bounded unreachable chunks, so physical reclamation remains a separate storage concern.

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