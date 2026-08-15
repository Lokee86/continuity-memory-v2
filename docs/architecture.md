# Architecture

Parent index: [Documentation index](INDEX.md)

## Purpose

This document owns the current Continuity Memory v2 implementation boundaries, state ownership, lifecycle, and code map.

## Overview

The repository currently implements one physical CVA container plus three concrete data owners:

```text
Cva
├── Container
│   └── global_version: u64
├── Archive
│   ├── archive_version: u64
│   └── conversation/session-local ancestry
├── PackedVectorStore
│   └── immutable numeric matrices
└── ArchiveVectorStore
    └── immutable row -> FragmentId bindings
```

`Cva` owns physical composition only. Archive owns source-history semantics. Packed vectors own numeric representation. Archive Vectors own the mapping between packed rows and Archive fragments. Embedding-profile and generation semantics are intentionally not part of Archive Vectors.

## Code root

Current implementation lives in `src/`; the corpus smoke harness is `examples/archive_roundtrip.rs`.

## Responsibilities

### Cva

`Cva` owns the single physical `Container` handle, writes required concrete format markers, and feeds one physical reopen scan to each concrete rebuild state. It is not a generalized semantic store, registry, root, or dependency engine.

### Container

`Container` owns fixed-header validation, opaque length-prefixed chunk I/O, stable `ChunkRef` addressing, explicit sync, and the append-only CVA-global `u64` version-ticket sequence. The global clock orders semantic mutations; it is not state identity or ancestry.

### Archive

`Archive` owns content-addressed conversation text, immutable nodes, conversation-local parent links, append-only branch/session-head revisions, immutable fragment ranges, the contiguous Archive-local `u64` watermark, and Archive-owned derived indexes.

`A=500` means the first 500 semantic Archive mutations. Archive-version adjacency is not conversation ancestry.

### PackedVectorStore

`PackedVectorStore` owns immutable dense matrix backing objects. Each object has a `VectorSchema`, row count, contiguous fixed-width row bytes, and SHA-256 content identity. Equal schema+matrix bytes deduplicate. The `lodestone-packed` representation supports non-zero dimensions and integer/floating scalar widths from 8-bit through 64-bit.

Packed vectors do not own embedding model identity, metrics, quantization meaning, row-to-domain identity, active generations, or semantic clocks.

### ArchiveVectorStore

`ArchiveVectorStore` owns immutable `ArchiveVectorSet` objects:

```text
ArchiveVectorSet
├── packed_vector_id
└── ordered FragmentId list
    row 0 -> fragment_ids[0]
    row 1 -> fragment_ids[1]
    ...
```

A set is content-addressed by the packed-matrix ID plus the ordered fragment mapping. Creation/reopen require an existing packed matrix, exact row-count equality, real Archive fragments, unique fragments within the set, and a valid content identity.

Archive Vectors do not own embedding profiles, model identity, metrics, normalization, Archive coverage watermarks, or active-generation state. Those belong to the profile/generation layer above this mapping.

Raw packed matrices and Archive-Vector sets are immutable backing objects and do not allocate semantic version tickets. A future vector-generation publication is the first vector-layer mutation expected to require semantic ordering.

## Does not own

Container does not interpret semantic records. `Cva` dispatches payloads but owns no database semantics. Archive does not own vector representation. Packed vectors do not know what rows mean. Archive Vectors do not know how rows were produced. Current indexes are derived acceleration state, not authority.

## Flow or lifecycle

### Semantic Archive write

```text
validate Archive record
    ↓
append immutable payload
    ↓
allocate global version G
    ↓
allocate next Archive version A
    ↓
append ArchiveRecordVersion { G, A, record_ref }
    ↓
update derived Archive lookup
```

No Archive-wide semantic parent pointer exists.

### Vector backing objects

```text
PackedVectors
    ↓ validate/store
PackedVectorId
    ↓ + ordered FragmentIds
ArchiveVectorSet
```

Neither step publishes an active retrieval generation.

### Open

```text
single streaming chunk walk
    ├── Container validates framing + global tickets
    └── Cva dispatches each payload
        ├── Archive rebuild
        ├── PackedVectorStore rebuild
        └── ArchiveVectorStore rebuild
                ↓
           after scan, validate row bindings against
           rebuilt Archive + PackedVectorStore
```

Archive-Vector mappings are retained transiently during reopen so cross-store validation does not require a second physical scan. Steady-state indexes retain metadata plus `ChunkRef`, not full vector matrices or fragment mappings.

## State or data ownership

Durable authorities:

```text
CVA header/chunks/global tickets          Container
Archive format/content/domain records     Archive
ArchiveRecordVersion metadata             Archive
Packed-vector format/matrix objects       PackedVectorStore
Archive-vector format/mapping objects     ArchiveVectorStore
```

Derived process state:

```text
Archive compact indexes                    Archive
PackedVectorId -> info + ChunkRef          PackedVectorStore
ArchiveVectorId -> info + ChunkRef         ArchiveVectorStore
```

A historical whole-Archive cut is identified by Archive version. General `ArchiveView` materialization remains unimplemented.

## Invariants and safety boundaries

- Global and Archive versions are exact integer ordering only.
- Conversation ancestry is node-local and never inferred from physical/version order.
- Unversioned Archive semantic payloads are inert.
- Packed matrices are immutable and content-addressed.
- Archive-Vector identity includes the packed matrix and ordered fragment mapping.
- Archive-Vector row count must exactly match packed-matrix row count.
- Archive-Vector mappings reference only existing, unique Archive fragments.
- Profile/generation semantics remain above Archive Vectors.
- Reopen uses one physical CVA scan shared by all current concrete stores.

See [architectural invariants](invariants.md).

## Code map

| Responsibility | Primary code |
| --- | --- |
| CVA composition/public operations | `src/cva.rs`, `src/cva_packed_vectors.rs`, `src/cva_archive_vectors.rs`, `src/cva_error.rs` |
| CVA header/chunks/global ordering | `src/container.rs`, `src/container_scan.rs`, `src/container_version.rs` |
| Archive semantics/history | `src/archive*.rs`, `src/fragment*.rs` |
| Packed-vector backing objects | `src/packed_vector_*.rs` |
| Archive-Vector row bindings | `src/archive_vector_*.rs` |
| History tests | `src/history_tests.rs` |
| Vector backing/binding tests | `src/packed_vector_tests.rs`, `src/archive_vector_tests.rs` |
| Corpus smoke | `examples/archive_roundtrip.rs` |

## Tests

Focused tests cover Archive clocks/history, fragments/content, packed-vector round trip/dedupe/corruption, and Archive-Vector mapping/reference/corruption rules. The prepared corpus smoke verifies complete Archive reconstruction across reopen.

## Related docs

- [Storage format](storage-format.md)
- [Rust API](api.md)
- [Architectural invariants](invariants.md)
- [Versioning and rollback plan](version-history-plan.md)
- [ADR 0005](decisions/0005-cva-composition-and-packed-vector-objects.md)
- [ADR 0006](decisions/0006-archive-vector-row-bindings.md)

## Notes

Whole-CVA restore across several future semantic databases remains a separate design problem. No every-write global state manifest is introduced here.
