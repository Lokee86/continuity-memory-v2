# Continuity Memory v2 Maintainer Map

Parent index: [Documentation index](INDEX.md)

## Purpose

This map routes common change intents to canonical documentation, implementation boundaries, and verification surfaces.

## Overview

Use this when ownership is unclear. It does not replace focused architecture/reference documentation.

## Change-area routing

| Change area | Canonical documentation | Primary implementation boundary | Verification |
| --- | --- | --- | --- |
| CVA header/chunks/file lifecycle | [Storage format](storage-format.md), [Architecture](architecture.md) | `src/container.rs`, `src/container_scan.rs` | `src/container_tests.rs` |
| Global CVA version ordering | [Architecture](architecture.md), [Storage format](storage-format.md) | `src/container_version.rs` | global-version tests |
| Archive-local watermark/record metadata | [Architecture](architecture.md), [ADR 0003](decisions/0003-layered-version-clocks-and-local-ancestry.md) | `src/archive_history.rs`, `src/archive_history_*` | `src/history_tests.rs` |
| Conversation node ancestry | [Architecture](architecture.md), [Rust API](api.md) | `src/archive.rs`, `src/archive_store.rs`, `src/archive_model.rs` | Archive/history tests, corpus smoke |
| Branch/session head revisions and old-session revival | [Architecture](architecture.md), [ADR 0003](decisions/0003-layered-version-clocks-and-local-ancestry.md) | `src/archive.rs`, `src/archive_history.rs` | `branch_heads_are_append_only_revisions`, `old_conversation_point_can_start_a_new_local_branch` |
| Content addressing/body reuse | [Architecture](architecture.md), [Storage format](storage-format.md) | `src/archive_store.rs`, `src/archive_codec.rs` | `src/archive_tests.rs` |
| Fragment identity/windows/tails | [Architecture](architecture.md), [Storage format](storage-format.md) | `src/fragment_*` | `src/fragment_tests.rs` |
| Reopen reconstruction/indexing/checkpoint acceleration | [Architecture](architecture.md), [Roadmap](roadmap.md), [Current limitations](current-limitations.md) | `src/container.rs`, `src/archive_rebuild.rs`, `src/archive_lookup.rs`, `src/archive_record_index.rs`, `src/archive_object_index.rs` | reopen tests + heap/startup profiling |
| Whole-CVA restore across databases | [Versioning plan](version-history-plan.md) | not implemented | future multi-store recovery suite |
| Additional semantic databases | [Roadmap](roadmap.md), [ADR 0001](decisions/0001-purpose-built-database-ownership.md) | not implemented | future store-local suites |
| Documentation governance | [Documentation policy](documentation-policy.md) | `docs-standard.json`, `AGENTS.md`, `docs/` | shared documentation checker |

## Boundaries

- Container owns physical storage and global ordering only.
- Archive owns Archive-local ordering and conversation/session semantics.
- Archive version adjacency does not imply conversation ancestry.
- Node parent links and branch/session revisions own local conversation history.
- Future stores retain independent authority even when they share the CVA global clock.

## Related docs

- [Architecture](architecture.md)
- [Documentation coverage](documentation-coverage.md)
- [Behavioral contracts](behavioral-contracts.md)
- [Roadmap](roadmap.md)

## Notes

Update this map when new stores or the shared runtime gain concrete implementation roots.