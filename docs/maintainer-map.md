# Continuity Memory v2 Maintainer Map

Parent index: [Documentation index](INDEX.md)

## Purpose

This map routes common change intents to canonical documentation, implementation boundaries, and verification surfaces.

## Overview

Use this when ownership is unclear. It does not replace focused architecture/reference documentation.

## Change-area routing

| Change area | Canonical documentation | Primary implementation boundary | Verification |
| --- | --- | --- | --- |
| CVA composition / single physical owner | [Architecture](architecture.md), [ADR 0005](decisions/0005-cva-composition-and-packed-vector-objects.md) | `src/cva.rs`, `src/cva_lifecycle.rs`, `src/cva_*.rs`, `src/cva_error.rs` | concrete-store reopen tests |
| CVA header/chunks/file lifecycle | [Storage format](storage-format.md), [Architecture](architecture.md) | `src/container.rs`, `src/container_scan.rs` | `src/container_tests.rs` |
| Global CVA version ordering | [Architecture](architecture.md), [Storage format](storage-format.md) | `src/container_version.rs` | global-version tests |
| Archive-local watermark/record metadata | [Architecture](architecture.md), [ADR 0003](decisions/0003-layered-version-clocks-and-local-ancestry.md) | `src/archive_history.rs`, `src/archive_history_*` | `src/history_tests.rs` |
| Conversation node ancestry | [Architecture](architecture.md), [Rust API](api.md) | `src/archive.rs`, `src/archive_store.rs`, `src/archive_model.rs` | Archive/history tests, corpus smoke |
| Branch/session head revisions and old-session revival | [Architecture](architecture.md), [ADR 0003](decisions/0003-layered-version-clocks-and-local-ancestry.md) | `src/archive.rs`, `src/archive_history.rs` | `branch_heads_are_append_only_revisions`, `old_conversation_point_can_start_a_new_local_branch` |
| Content addressing/body reuse | [Architecture](architecture.md), [Storage format](storage-format.md) | `src/archive_store.rs`, `src/archive_codec.rs` | `src/archive_tests.rs` |
| Fragment identity/windows/tails | [Architecture](architecture.md), [Storage format](storage-format.md) | `src/fragment_*` | `src/fragment_tests.rs` |
| Packed-vector representation/storage | [Storage format](storage-format.md), [ADR 0005](decisions/0005-cva-composition-and-packed-vector-objects.md) | `src/packed_vector_*`, Lodestone `crates/packed` | `src/packed_vector_tests.rs`, Lodestone packed tests |
| Archive-Vector row bindings | [Storage format](storage-format.md), [ADR 0006](decisions/0006-archive-vector-row-bindings.md) | `src/archive_vector_*`, `src/cva_archive_vectors.rs` | `src/archive_vector_tests.rs` |
| Embedding endpoint / compatibility contract | [Rust API](api.md), [ADR 0007](decisions/0007-compatibility-profiles-and-vector-generations.md) | `src/embedding_endpoint.rs`, `src/compatibility_profile_*`, `src/cva_compatibility_profiles.rs` | `src/compatibility_profile_tests.rs` |
| Vector-generation publication/history | [Architecture](architecture.md), [Versioning plan](version-history-plan.md), [ADR 0007](decisions/0007-compatibility-profiles-and-vector-generations.md) | `src/vector_generation_*`, `src/cva_vector_generations.rs` | generation tests + corpus vector smoke |
| Exact semantic retrieval | [Architecture](architecture.md), [Rust API](api.md), [Current limitations](current-limitations.md) | `src/semantic_search*.rs` | `src/semantic_search_tests.rs`, corpus vector/retrieval smoke |
| Reopen reconstruction/indexing/checkpoint acceleration | [Architecture](architecture.md), [Roadmap](roadmap.md), [Current limitations](current-limitations.md) | `src/cva_lifecycle.rs`, `src/container.rs`, concrete rebuild states, Archive indexes | reopen tests + heap/startup profiling |
| Whole-CVA restore across databases | [Versioning plan](version-history-plan.md) | not implemented | future multi-store recovery suite |
| Additional semantic databases | [Roadmap](roadmap.md), [ADR 0001](decisions/0001-purpose-built-database-ownership.md) | not implemented | future store-local suites |
| Documentation governance | [Documentation policy](documentation-policy.md) | `docs-standard.json`, `AGENTS.md`, `docs/` | shared documentation checker |

## Boundaries

- `Cva` owns the single Container handle and explicit concrete-store scan dispatch; it owns no semantic dependency graph.
- Container owns physical storage and global ordering only.
- Archive owns Archive-local ordering and conversation/session semantics.
- Packed-vector storage owns immutable matrix identity/shape only.
- Archive-Vector storage owns row-to-Archive-fragment identity only.
- Compatibility Profiles own immutable endpoint-independent vector-space compatibility contracts and reference probes; provider/model provenance is outside this owner.
- Vector Generations own profile/population activation, Archive coverage, and the vector-local semantic watermark.
- Archive and vector-local version adjacency do not imply ancestry or cross-store dependency identity.
- Node parent links and branch/session revisions own local conversation history.
- Future stores retain independent authority even when they share the CVA global clock.

## Related docs

- [Architecture](architecture.md)
- [Documentation coverage](documentation-coverage.md)
- [Behavioral contracts](behavioral-contracts.md)
- [Roadmap](roadmap.md)

## Notes

Update this map when new stores or the shared runtime gain concrete implementation roots.