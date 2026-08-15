# Development

Parent index: [Documentation index](INDEX.md)

## Purpose

This document owns the current repository workflow, layout, verification commands, and corpus smoke harness.

## Overview

The repository is a single Rust library crate. Current work should preserve explicit storage ownership and keep physical-container mechanics separate from Archive semantics.

## Workflow or repository boundary

```text
Cargo.toml / Cargo.lock          crate definition and locked dependencies
src/cva*.rs                     single-Container composition and public CVA lifecycle
src/container*.rs               physical CVA substrate
src/archive*.rs                 Archive records, compact lookup, validation, semantic operations
src/fragment*.rs                durable fragment ranges and materialization
src/packed_vector*.rs           immutable packed-vector objects/rebuild/index
src/archive_vector*.rs          immutable packed-row to FragmentId bindings
src/lib.rs                      public exports
examples/archive_roundtrip.rs    prepared-corpus reopen smoke
examples/archive_open_profile.rs standalone allocator/open-time benchmark

docs/                            current docs, plans, limits, decisions
```

Do not copy large portions of the previous Continuity implementation into this repository as a migration shortcut. Reuse must preserve the boundaries in [architecture](architecture.md).

## Commands

Normal verification:

```text
cargo fmt --check
cargo check
cargo test
```

Documentation verification:

```text
python ../engineering-standards/tools/docs_policy/check.py --repo .
```

Prepared graph-corpus round trip:

```text
cargo run --example archive_roundtrip -- <graph.jsonl> <output.cva>
```

The example deletes an existing output path before creating the smoke CVA. It is a test harness, not the production import interface.

Prepared-Archive open benchmark:

```text
cargo run --release --example archive_open_profile -- <archive.cva> [runs]
```

The benchmark is standalone and does not instrument the production `Cva::open` path. It reports median/p90 open time plus allocator-tracked retained and peak bytes.

### Prepared-corpus current measurements — 2026-08-14

The current prepared corpus contains 12 conversations and produces a `2,197,522`-byte CVA on the required Archive + packed-vector + Archive-Vector marker format, with `1,875` semantic Archive records: `1,559` nodes, `35` current branches, `281` fragments, and `1,553` content objects. The fixture currently contains zero packed-vector matrices and zero Archive-Vector sets.

A release-mode 50-run warm-cache `Cva::open` benchmark measured `25.229 ms` median and `28.549 ms` p90. Allocator-tracked retained heap was `752,907` bytes and peak additional heap `884,586` bytes.

A current-format cold-cache sample has not yet been recorded. Larger archives must be measured with repeated cache eviction plus one open per sample before drawing conclusions about cold-open scaling or adding persistent Archive checkpoints.

## Failure modes

- `cargo fmt --check` fails when Rust formatting drift exists.
- `cargo check` catches compilation/type errors without running tests.
- `cargo test` owns current behavioral verification.
- The documentation checker catches structural policy, required-path, index, link, coverage, and configured change-rule issues; it does not prove semantic correctness.
- The round-trip example fails when reopened counts, branch paths, content, or Archive reconstruction disagree with the imported fixture.

## Code map

Detailed responsibility-to-file mapping lives in [architecture](architecture.md). Invariant-to-test mapping lives in [behavioral contracts](behavioral-contracts.md).

## Related docs

- [Architecture](architecture.md)
- [Maintainer map](maintainer-map.md)
- [Documentation coverage](documentation-coverage.md)
- [Behavioral contracts](behavioral-contracts.md)

## Notes

A production ChatGPT/Hermes importer is not implemented in this repository. `archive_roundtrip` consumes an already prepared graph JSONL fixture.