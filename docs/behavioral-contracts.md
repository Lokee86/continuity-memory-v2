# Behavioral Contracts

Parent index: [Documentation index](INDEX.md)

## Purpose

This document maps critical current behavioral invariants to focused tests and smoke verification.

## Overview

The matrix covers implemented behavior. Future cross-database restore and concurrency contracts remain explicit future requirements.

## Contract matrix

| Contract | Protection |
| --- | --- |
| New CVA header reopens | `container_tests::create_then_reopen_cva` |
| Opaque chunks retain stable references | `container_tests::append_then_read_chunks` |
| Global versions survive reopen | `container_tests::global_versions_survive_reopen` |
| Truncated/non-CVA files are rejected | `container_tests::*truncated*`, `reject_non_cva_file` |
| Archive requires current format marker | `history_tests::archive_rejects_container_without_archive_format_marker` |
| Unrelated conversations share ordering clocks but not ancestry | `history_tests::unrelated_conversations_share_clocks_not_ancestry` |
| Global and Archive clocks can diverge cleanly | `history_tests::global_and_archive_clocks_are_independent` |
| Archive-local watermark survives reopen | `history_tests::archive_versions_survive_reopen` |
| Branch/session heads are append-only revisions with historical lookup | `history_tests::branch_heads_are_append_only_revisions` |
| Existing branch heads cannot jump backward to an ancestor | `history_tests::existing_branch_head_cannot_jump_backwards` |
| An old conversation point can seed a new local branch without Archive rollback | `history_tests::old_conversation_point_can_start_a_new_local_branch` |
| Shared prefixes/content bodies deduplicate | `archive_tests::shared_branch_prefix_and_content_are_stored_once` |
| Identical node append is idempotent | `archive_tests::identical_node_append_is_idempotent` |
| Fragment windows/tails remain append-only and branch-neutral | `fragment_tests::*` |
| Prepared corpus round-trips branches/content/fragments | `examples/archive_roundtrip.rs` |

## Future contracts

Before concurrent writers are supported, tests must prove that unrelated sessions do not serialize on a semantic Archive head and that global/file-position reservation is the only required shared ordering boundary.

Before whole-CVA rollback ships, tests must define timeline selection across Archive, Memories, Graph, and vector domains without turning every write into a global state publication.

Before compact indexing/checkpoints replace current maps, tests/benchmarks must prove semantic equivalence and measure open-time/heap behavior.

## Related docs

- [Architecture](architecture.md)
- [Architectural invariants](invariants.md)
- [Versioning and rollback plan](version-history-plan.md)
- [Development](development.md)

## Notes

A tracked integer watermark is not by itself a historical materialization API; tests distinguish stored ordering metadata from features not yet exposed.