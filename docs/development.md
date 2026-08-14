# Development

Parent index: [Documentation index](INDEX.md)

## Purpose

This document owns the current repository workflow, layout, verification commands, and corpus smoke harness.

## Overview

The repository is a single Rust library crate. Current work should preserve explicit storage ownership and keep physical-container mechanics separate from Archive semantics.

## Workflow or repository boundary

```text
Cargo.toml / Cargo.lock          crate definition and locked dependencies
src/container*.rs               physical CVA substrate
src/archive*.rs                 Archive records, compact lookup, validation, public operations
src/fragment*.rs                durable fragment ranges and materialization
src/lib.rs                      public exports
examples/archive_roundtrip.rs   prepared-corpus reopen smoke
examples/archive_open_profile.rs allocator/open-time profiling harness
src/archive_profile.rs          opt-in Archive index/open diagnostics

docs/                           current docs, plans, limits, decisions
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

Prepared-Archive open profiling:

```text
cargo run --release --example archive_open_profile -- <archive.cva> [runs]
```

The profiling example reports median/p90 open time plus allocator-tracked retained and peak bytes. Setting `CONTINUITY_PROFILE_ARCHIVE_OPEN` also prints single-pass scan/reconstruction and validation timing, dense-record/lookup capacities, retained string capacity, and a conservative known-heap estimate. The diagnostics are opt-in and do not alter persistent format or Archive authority.

### Prepared-corpus pre-compaction baseline — 2026-08-14

Release-mode warm-cache profiling of the 12-conversation `2,197,482`-byte corpus measured `7,179` physical chunks and `1,875` semantic Archive records. Across 50 untraced opens, median `Archive::open()` was `83.1 ms` and p90 was `84.8 ms`; allocator-tracked retained heap was `1,014,811` bytes and peak additional heap was `1,146,558` bytes.

A separate 25-run traced sample measured about `42.0 ms` in `Container::open`, `40.6 ms` in Archive rebuild, and `4.2 ms` in reference validation. Temporary deeper instrumentation showed Container performing two framing walks of about `10.4 ms` each plus about `20.7 ms` reading payloads for global-version reconstruction; Archive then spent about `33.1 ms` scanning/rereading chunks and `7.3 ms` replaying semantic records. The deep Container timing was measurement-only and was removed after recording this baseline.

Pre-compaction derived capacities were `1553/1792` contents, `1559/1792` nodes, `35/56` branches, and `281/448` fragments. Node composite lookup keys retained `224,496` bytes of string capacity even though their actual encoded key text was only `113,807` bytes; node value strings retained another `178,556` bytes. The node-map structural/string estimate was about `690 KB`, making it the dominant derived-index heap consumer.

### Prepared-corpus compact-index result — 2026-08-14

The composite node/branch string-key maps were replaced by dense records plus one-machine-word open-addressed lookup slots; fragments use the same pattern. Content retains a direct fixed-width hash table after measurement showed the indirect representation was larger for `ContentId -> ChunkRef`.

Allocator-tracked retained heap is now `752,905` bytes and peak additional heap is `884,584` bytes, reductions of `261,906` and `261,974` bytes respectively from the pre-compaction baseline. Current known index-heap estimates are `86,016` bytes for content, `489,852` for nodes, `9,412` for branches, and `87,692` for fragments.

Open time did not improve materially from index compaction alone: three 50-run warm-cache batches had median opens of `83.3`, `92.9`, and `89.6 ms`. A traced sample measured about `40.3 ms` in Container open, `39.0 ms` in Archive rebuild, and `2.7 ms` in validation; Archive scan/reread was about `32.3 ms` while semantic replay was about `6.6 ms`. Those measurements motivated the single-pass reopen change below.

### Prepared-corpus single-pass reopen result — 2026-08-14

Container open and Archive reconstruction now share one streaming chunk walk. Container validates framing and observes global version tickets; Archive consumes the same payload bytes, keeps unversioned semantic records pending, and activates them only when their `ArchiveRecordVersion` appears. This removes the previous framing rescans and semantic-record rereads without moving Archive semantics into Container.

Three 50-run warm-cache batches measured median opens of `25.0`, `25.3`, and `26.8 ms`; the median of those medians is `25.3 ms`, with a representative p90 of `27.2 ms`. Allocator-tracked retained heap remains effectively unchanged at `752,903` bytes and peak additional heap at `884,582` bytes. A 25-run traced sample measured about `24.6 ms` in the combined streaming scan/reconstruction path and `3.0 ms` in reference validation. Compared with the original `83.1 ms` baseline, median reopen time is about 70% lower. Persistent checkpointing is not currently justified by this corpus.

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