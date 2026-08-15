# Development

Parent index: [Documentation index](INDEX.md)

## Purpose

This document owns repository workflow, layout, verification commands, corpus smoke harnesses, and current measurements.

## Overview

Development uses deterministic local tests and simulated embedding execution. Model-switchboard routing, encrypted credential persistence, and auth-header attachment are implemented; direct provider HTTP transport and Codex device-code acquisition/refresh remain outside the current slice. A separate repo-local CLI exposes current bring-up operations without installation.

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

A current-format cold-cache sample has not yet been recorded. Larger realistic-dimension vector populations require repeated cache eviction plus one open per sample before conclusions about cold scaling, segmentation, or checkpoints.

## Failure modes

- `cargo fmt --check` catches Rust formatting drift.
- `cargo check` catches type/compile errors.
- Core `cargo test` owns focused library behavior, including config replacement, encrypted credential round trips/tamper rejection, switchboard auth binding, and master-key generation/reload/redaction.
- The separate CLI fmt/check/test commands verify that the detachable package compiles only against the public library surface.
- Command-level CLI smoke tests exercise CVA create/info/verify, graph import/archive inspection, simulated profile/vector/search execution, and config show/verify without installing the binary.
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

Direct provider HTTP adapters, Codex device-code acquisition/token refresh, and production import interfaces are not implemented in this repository.