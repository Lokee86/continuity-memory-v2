# Documentation Coverage

Parent index: [Documentation index](INDEX.md)

## Purpose

This document maps current implementation boundaries to canonical documentation owners.

## Overview

Coverage remains intentionally small while the rebuild is early.

## Implementation coverage

| Implementation | Responsibility | Canonical owners |
| --- | --- | --- |
| `src/cva.rs`, `src/cva_packed_vectors.rs`, `src/cva_archive_vectors.rs`, `src/cva_error.rs` | Single-Container composition, concrete-store scan dispatch, public lifecycle/API | [Architecture](architecture.md), [Rust API](api.md), [ADR 0005](decisions/0005-cva-composition-and-packed-vector-objects.md) |
| `src/container.rs` | CVA header/chunks/read/append/sync | [Architecture](architecture.md), [Storage format](storage-format.md), [Rust API](api.md) |
| `src/container_scan.rs` | Single-pass physical payload scan and global-ticket observation | [Architecture](architecture.md), [Storage format](storage-format.md), [Development](development.md) |
| `src/container_version.rs` | CVA-global monotonic version tickets | [Architecture](architecture.md), [Storage format](storage-format.md) |
| `src/container_error.rs` | Physical/version errors | [Storage format](storage-format.md), [Rust API](api.md) |
| `src/container_tests.rs` | Container/version verification | [Behavioral contracts](behavioral-contracts.md) |
| `src/archive.rs` | Archive mutation/read and branch-head revisions | [Architecture](architecture.md), [Rust API](api.md) |
| `src/archive_history.rs` | Archive-local watermark and dual-version metadata | [Architecture](architecture.md), [Versioning plan](version-history-plan.md), [Rust API](api.md) |
| `src/archive_history_codec.rs` | Archive format + record-version metadata codec | [Storage format](storage-format.md) |
| `src/archive_history_model.rs` | `ArchiveRecordVersion` model | [Storage format](storage-format.md), [Rust API](api.md) |
| `src/archive_model.rs` | Archive public semantic models | [Architecture](architecture.md), [Rust API](api.md) |
| `src/archive_codec.rs` | Node/content/branch/fragment codecs | [Storage format](storage-format.md) |
| `src/archive_rebuild.rs` | Single-pass reopen reconstruction and semantic visibility activation | [Architecture](architecture.md), [Storage format](storage-format.md), [Current limitations](current-limitations.md) |
| `src/archive_store.rs` | Content access, branch traversal, and current-state validation | [Architecture](architecture.md), [Current limitations](current-limitations.md) |
| `src/archive_error.rs` | Archive error contract | [Rust API](api.md) |
| `src/archive_tests.rs` | Archive dedupe/idempotency | [Behavioral contracts](behavioral-contracts.md) |
| `src/fragment_*` | Fragment identity/materialization/storage/tests | [Architecture](architecture.md), [Storage format](storage-format.md), [Behavioral contracts](behavioral-contracts.md) |
| `src/history_tests.rs` | Dual clocks, independent conversations, branch revisions/revival | [Behavioral contracts](behavioral-contracts.md), [ADR 0003](decisions/0003-layered-version-clocks-and-local-ancestry.md) |
| `src/packed_vector_*` | Packed-vector format, content identity, derived lookup, reopen, tests | [Architecture](architecture.md), [Storage format](storage-format.md), [Rust API](api.md), [ADR 0005](decisions/0005-cva-composition-and-packed-vector-objects.md) |
| `src/archive_vector_*` | Row-to-FragmentId binding format, identity, reference validation, reopen, tests | [Architecture](architecture.md), [Storage format](storage-format.md), [Rust API](api.md), [ADR 0006](decisions/0006-archive-vector-row-bindings.md) |
| `src/lib.rs` | Public crate exports | [Rust API](api.md) |
| `examples/archive_roundtrip.rs` | Prepared-corpus reopen smoke | [Development](development.md), [Behavioral contracts](behavioral-contracts.md) |
| `examples/archive_open_profile.rs` | Standalone open-time/allocator benchmark | [Development](development.md) |

## Coverage rules

New production modules require canonical owners. New persistence records update storage-format documentation. New stateful/recovery boundaries update architecture and behavioral contracts in the same change.

## Related docs

- [Documentation policy](documentation-policy.md)
- [Maintainer map](maintainer-map.md)
- [Architecture](architecture.md)
- [Behavioral contracts](behavioral-contracts.md)

## Notes

Future components remain planning-only until code exists.