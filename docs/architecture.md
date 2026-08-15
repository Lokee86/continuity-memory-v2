# Architecture

Parent index: [Documentation index](INDEX.md)

## Purpose

This document owns the current Continuity Memory v2 implementation boundaries, state ownership, lifecycle, and code map.

## Overview

The repository currently implements the physical CVA container, Archive, and an immutable packed-vector backing store. `Cva` owns physical composition without merging database semantics:

```text
Cva
├── Container
│   └── global_version: u64
├── Archive DB
│   ├── archive_version: u64
│   └── conversation/session-local ancestry
└── PackedVectorStore
    └── immutable content-addressed matrices
```

The clocks provide ordering and historical cuts for meaningful semantic mutations. They do not define semantic parentage. Raw packed-vector matrices are backing objects and do not consume semantic versions before embedding-profile/generation ownership exists.

## Code root
Current implementation lives in `src/`; the corpus smoke harness is `examples/archive_roundtrip.rs`.
## Responsibilities

### Cva

`Cva` owns the single physical `Container` handle and composes concrete database states. Creation writes the required concrete format markers. Reopen performs one physical scan and explicitly feeds each payload to Archive and packed-vector rebuild logic. `Cva` is not a generalized semantic store, database registry, or dependency owner.

### Container

`Container` owns:

- fixed CVA header validation;
- opaque length-prefixed chunk append/read;
- stable `ChunkRef` addressing;
- explicit `sync_all`;
- one append-only global `u64` version-ticket sequence.

The global clock orders durable mutations across future databases. It is not a CVA state root or dependency identity.

### Archive

`Archive` owns:

- content-addressed conversation text;
- immutable nodes with conversation-local parent links;
- append-only branch/session head revisions;
- immutable fragment ranges;
- one contiguous Archive-local `u64` version sequence;
- mapping each semantic Archive record to both global and Archive versions;
- current branch-head reconstruction and historical branch-head queries;
- Archive-local validation and derived indexes.

The Archive-local clock is a watermark: `A=500` means the first 500 semantic Archive mutations, not a parent relationship.

### PackedVectorStore

`PackedVectorStore` owns immutable dense matrix backing objects. Each object has a `VectorSchema` (non-zero dimensions plus scalar representation), a row count, contiguous fixed-width row bytes, and a SHA-256 content identity. Equal schema+matrix bytes deduplicate. The current schema layer comes from the lightweight `lodestone-packed` crate and supports integer/floating scalar widths from 8-bit through 64-bit.

Packed vectors deliberately do not own embedding model identity, quantization meaning, similarity metric, row-to-Archive mapping, active generations, or semantic clocks. Those belong to the embedding-profile and Archive-Vector slices that follow.

## Does not own

The container does not interpret Archive, Memories, Graph, vector, session, or dependency semantics. `Cva` dispatches physical payloads but owns none of those semantics. Archive does not impose ancestry between unrelated conversations merely because their writes are physically ordered. Packed-vector storage does not interpret what rows mean. Current lookup structures are derived acceleration state, not authority.

## Flow or lifecycle

### Semantic Archive write

```text
validate domain record
    ↓
append immutable semantic payload
    ↓
allocate global version G
    ↓
allocate next Archive version A
    ↓
append ArchiveRecordVersion { G, A, record_ref }
    ↓
update derived current lookup
```

The metadata record contains no Archive-wide parent pointer.

A failed operation may consume a global version ticket without producing an Archive version. Archive versions themselves remain contiguous.

### Conversation and branch history

Node `parent_id` links own conversation-local ancestry; unrelated conversations may interleave in global/Archive ordering without acquiring ancestry from one another. `Branch { conversation_id, id, leaf_node_id, canonical }` is an append-only logical head revision. `branch_at(..., A)` resolves the latest revision visible through Archive version `A`.

Continuing an old conversation point creates a new branch identity and descendants from that old node. The Archive watermark continues forward; no whole-Archive rollback occurs and unrelated conversations are unaffected.

### Open

```text
single streaming chunk walk
    ├── Container validates framing + global version tickets
    └── Cva dispatches the same payload
        ├── Archive: format/content/semantic/version records
        │       ↓
        │  pending semantic records wait for version metadata
        │       ↓
        │  validate contiguous Archive versions + activate records
        │       ↓
        │  validate references/fragments
        └── PackedVectorStore: format/object records
                ↓
           validate matrix shape + recompute content identity
                ↓
           retain metadata + ChunkRef, not matrix bytes
```

Container supplies physical framing and the current global watermark but never interprets database meaning. An interrupted Archive node/branch/fragment payload without `ArchiveRecordVersion` remains inert. Packed matrices are immutable backing objects and are independently valid once their complete object chunk exists.

## State or data ownership

Durable authorities:

```text
CVA header/chunks/global tickets         Container
Archive format marker                    Archive
Archive content objects                  Archive
Archive node/branch/fragment records     Archive
ArchiveRecordVersion metadata            Archive
Packed-vector format marker              PackedVectorStore
Packed-vector matrix objects             PackedVectorStore
```

Derived process state:

```text
ContentId -> ChunkRef                  fixed-width content lookup
NodeIndex                              dense Node records + compact hash-to-index slots
BranchIndex                            dense current Branch records + compact hash-to-index slots
FragmentIndex                          dense Fragment records + compact hash-to-index slots
Vec<ArchiveRecordVersion>              dense Archive mutation metadata
PackedVectorId -> info + ChunkRef       packed-vector object lookup
```

Node and branch lookups hash `(conversation_id, id)` without materializing composite key strings; exact identity is checked against dense records. Content keeps a direct fixed-width lookup. A historical whole-Archive cut is identified by Archive version; general `ArchiveView` materialization is not yet exposed.

## Invariants and safety boundaries

- Global and Archive versions are exact integers used for ordering only.
- Archive versions are contiguous within Archive; global versions may have gaps between Archive mutations.
- No Archive-wide semantic parent chain exists.
- Node ancestry never crosses conversation IDs.
- Branch/session revisions do not rewrite older revisions.
- Reviving an old conversation advances a local branch rather than rewinding the Archive.
- Content bodies remain content-addressed and fragments remain branch-neutral ranges.
- Normal Archive writes do not inspect or republish future Memories/Graph/vector state.
- Raw packed-vector objects do not advance the global or Archive semantic clocks.
- Packed-vector identity includes schema plus exact matrix bytes; row meaning is external to the packed store.
- Reopen performs one physical CVA scan shared by the current concrete stores.

See [architectural invariants](invariants.md).

## Code map

| Responsibility | Primary code |
| --- | --- |
| CVA composition/public operations | `src/cva.rs`, `src/cva_error.rs` |
| CVA header/chunks/streaming scan | `src/container.rs`, `src/container_scan.rs` |
| Global version clock | `src/container_version.rs` |
| Archive semantic operations | `src/archive.rs` |
| Archive record-version clocks/history | `src/archive_history.rs` |
| Archive version codec/model | `src/archive_history_codec.rs`, `src/archive_history_model.rs` |
| Archive models/codecs | `src/archive_model.rs`, `src/archive_codec.rs` |
| Reopen reconstruction/current indexes | `src/archive_rebuild.rs`, `src/archive_store.rs`, `src/archive_lookup.rs`, `src/archive_record_index.rs`, `src/archive_object_index.rs` |
| Fragments | `src/fragment_model.rs`, `src/fragmenter.rs`, `src/fragment_store.rs` |
| Packed-vector format/store/rebuild | `src/packed_vector_codec.rs`, `src/packed_vector_model.rs`, `src/packed_vector_store.rs`, `src/packed_vector_rebuild.rs` |
| History/concurrency semantics tests | `src/history_tests.rs` |
| Packed-vector tests | `src/packed_vector_tests.rs` |
| Corpus smoke | `examples/archive_roundtrip.rs` |
## Tests
Focused tests cover layered clocks, conversation-local ancestry/branch history, inert unversioned Archive records, content/fragment invariants, packed-vector round trip/dedupe/clock neutrality, and corrupt-vector rejection. The prepared corpus smoke verifies complete branch/content reconstruction across reopen.

## Related docs

- [Storage format](storage-format.md)
- [Rust API](api.md)
- [Architectural invariants](invariants.md)
- [Versioning and rollback plan](version-history-plan.md)
- [ADR 0003](decisions/0003-layered-version-clocks-and-local-ancestry.md)
- [ADR 0005](decisions/0005-cva-composition-and-packed-vector-objects.md)
## Notes

Whole-CVA restore across several future databases remains a separate design problem. The clocks implemented here deliberately avoid solving it with an every-write global state manifest.