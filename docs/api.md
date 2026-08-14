# Rust API Reference

Parent index: [Documentation index](INDEX.md)

## Purpose

This document owns the current public Rust library surface exposed by `continuity_memory`.

## Overview

The public API exposes the physical `Container`, Archive, Archive/fragment models, dual version metadata, chunk references, and errors. It remains development-stage.

## Exact contract

### Container

Public types:

```text
Container
ContainerError
ChunkRef { offset: u64, len: u64 }
FormatVersion { major: u16, minor: u16 }
```

Public operations include create/open, opaque append/read/chunk enumeration, path/format inspection, `sync()`, and `latest_version()` for the current CVA-global `u64` clock.

### Archive version metadata

```text
ArchiveRecordVersion {
    global_version: u64,
    archive_version: u64,
    record: ChunkRef,
}
```

`global_version` orders the mutation in the CVA. `archive_version` is the contiguous Archive-local watermark. Neither field is a semantic parent pointer.

### Archive

Public models:

```text
ContentId([u8; 32])
Node
Branch
ResolvedTurn
ArchiveStats
FragmentId([u8; 32])
Fragment
FragmentConfig
ArchiveRecordVersion
```

Core operations:

- `Archive::create(path)` creates a new Archive and writes its format marker.
- `Archive::open(path)` performs one streaming physical scan that validates Container framing/global tickets while rebuilding only versioned Archive state, then validates references.
- `append_node(...)` appends an immutable conversation node and its dual-version metadata.
- `append_branch(branch)` appends a branch/session-head revision when that logical branch changed; an identical current revision is idempotent. An existing branch head may advance only to a descendant node; reviving an older point requires a new branch identity.
- `branch_turns(...)` resolves the current branch leaf through node parent links.
- `stats()` reports current derived counts.
- `sync()` flushes the container.

Version/history operations:

- `archive_version()` returns the latest Archive-local version, or `0` for no semantic Archive records.
- `record_versions()` returns retained Archive record-version metadata in Archive-version order.
- `record_version(A)` returns exact metadata for Archive version `A`.
- `branch_at(conversation_id, branch_id, A)` returns the latest revision of that branch visible through Archive version `A`.

The current API tracks every whole-Archive cut but does not yet expose a general materialized historical `ArchiveView`.

Fragment operations remain `materialize_path_fragments`, `materialize_branch_fragments`, `fragment_turns`, `fragment_text`, and deterministic `fragments()` enumeration.

## Defaults or precedence

`FragmentConfig::default()` is eight turns with two-turn overlap.

Node identity is `(conversation_id, node_id)`. Branch/session identity is `(conversation_id, branch_id)`. Repeated branch records with the same identity are revisions; the newest Archive version is current.

## Diagnostics or failure behavior

`ArchiveError` covers semantic reference failures, immutable-record conflicts, malformed content/fragments, missing or conflicting Archive format markers, invalid Archive record-version sequences, version exhaustion, and underlying container errors.

Durability remains explicit through `sync()`.

## Examples

Concurrent conversations may interleave:

```text
G1/A1  conversation A node
G2/A2  conversation B node
G3/A3  conversation A node
```

The clocks capture ordering; A's node parent links capture A's ancestry.

To revive an old conversation point, create another branch identity at that old node, append a descendant node, then append a new revision of that branch head.

## Related docs

- [Architecture](architecture.md)
- [Storage format](storage-format.md)
- [Behavioral contracts](behavioral-contracts.md)
- [Versioning and rollback plan](version-history-plan.md)

## Notes

No compatibility promise has yet been made for Rust method signatures or the bootstrap persistence format.