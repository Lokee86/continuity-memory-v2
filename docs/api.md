# Rust API Reference

Parent index: [Documentation index](INDEX.md)

## Purpose

This document owns the current public Rust library surface exposed by `continuity_memory`.

## Overview

The public API exposes `Cva` as the file/composition owner, the physical `Container`, Archive state/models, packed-vector types, dual Archive version metadata, chunk references, and errors. It remains development-stage.

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

### Cva

`Cva::create(path)` creates one CVA, initializes the concrete Archive and packed-vector format markers, and owns the single Container handle. `Cva::open(path)` performs one streaming physical scan and rebuilds both current concrete stores.

Archive mutation/read operations are exposed through `Cva`: `append_node`, `append_branch`, `branch_turns`, `branch_at`, fragment materialization/read operations, `stats`, Archive-version inspection, and `sync`. `archive()` returns read-only access to the Archive semantic state.

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

Core Archive operations through `Cva`:

- `append_node(...)` appends an immutable conversation node and its dual-version metadata.
- `append_branch(branch)` appends a branch/session-head revision when that logical branch changed; an identical current revision is idempotent. An existing branch head may advance only to a descendant node; reviving an older point requires a new branch identity.
- `branch_turns(...)` resolves the current branch leaf through node parent links.
- `stats()` reports current derived Archive counts.
- `sync()` flushes the CVA container.

Version/history operations:

- `archive_version()` returns the latest Archive-local version, or `0` for no semantic Archive records.
- `record_versions()` returns retained Archive record-version metadata in Archive-version order.
- `record_version(A)` returns exact metadata for Archive version `A`.
- `branch_at(conversation_id, branch_id, A)` returns the latest revision of that branch visible through Archive version `A`.

The current API tracks every whole-Archive cut but does not yet expose a general materialized historical `ArchiveView`.

Fragment operations remain `materialize_path_fragments`, `materialize_branch_fragments`, `fragment_turns`, `fragment_text`, and deterministic `fragments()` enumeration.

### Packed vectors

Public types:

```text
ScalarType
VectorSchema { dimensions, scalar }
PackedVectors
PackedVectorId([u8; 32])
PackedVectorInfo { id, schema, count, byte_len }
PackedVectorStats { objects, rows, matrix_bytes }
PackedVectorError
```

`ScalarType` currently covers signed/unsigned 8/16/32/64-bit integers plus f16, bf16, f32, and f64. `VectorSchema` accepts any non-zero `u32` dimension count. `PackedVectors` is a contiguous fixed-row byte matrix supplied by Lodestone's lightweight `lodestone-packed` crate.

`Cva::put_packed_vectors(packed)` content-addresses and deduplicates an immutable matrix inside the CVA and returns its `PackedVectorId`. `Cva::packed_vectors(id)` reads and validates the matrix. `packed_vector_infos()` and `packed_vector_stats()` inspect the derived object inventory.

Raw packed-vector objects have no embedding profile or row meaning and do not advance global/Archive semantic clocks.

## Defaults or precedence

`FragmentConfig::default()` is eight turns with two-turn overlap.

Node identity is `(conversation_id, node_id)`. Branch/session identity is `(conversation_id, branch_id)`. Repeated branch records with the same identity are revisions; the newest Archive version is current.

## Diagnostics or failure behavior

`ArchiveError` covers Archive semantic/reference failures. `PackedVectorError` covers packed-vector format/object failures. `CvaError` covers create/open/composition failures across Container, Archive, and packed-vector rebuild.

Durability remains explicit through `Cva::sync()`.

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

No compatibility promise has yet been made for Rust method signatures or the bootstrap persistence format. Embedding profiles, vector generations, Archive row bindings, and search APIs are the next vector-layer slices rather than responsibilities of `PackedVectors` itself.