# Development

Parent index: [Documentation index](INDEX.md)

## Purpose

This document owns repository workflow, layout, verification commands, corpus smoke harnesses, and current measurements.

## Overview

Development uses deterministic local tests plus optional live OpenAI-ready embedding execution. Model-switchboard routing, encrypted credential persistence, auth-header attachment, and direct embedding HTTP transport are implemented; General-model transport and Codex device-code acquisition/refresh remain outside the current slice. A separate repo-local CLI exposes current bring-up operations without installation.

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

Additional provider-native/local embedding adapters, General-model transport, Codex device-code acquisition/token refresh, and production import interfaces are not implemented in this repository.