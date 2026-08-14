# Architecture

Parent index: [Documentation index](INDEX.md)

## Purpose

This document owns the current Continuity Memory v2 implementation boundaries, state ownership, lifecycle, and code map.

## Overview

The repository currently implements a physical CVA container and the Archive database. The governing hierarchy is:

```text
CVA
└── global_version: u64
    └── Archive DB
        ├── archive_version: u64
        └── conversation/session-local ancestry
```

The clocks provide ordering and historical cuts. They do not define semantic parentage. Conversation ancestry is owned by Archive nodes and branch/session head revisions.

## Code root
Current implementation lives in `src/`; the corpus smoke harness is `examples/archive_roundtrip.rs`.
## Responsibilities

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

## Does not own

The container does not interpret Archive, Memories, Graph, vector, session, or dependency semantics. Archive does not impose ancestry between unrelated conversations merely because their writes are physically ordered. Current compact lookup structures are derived acceleration state, not authority.

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

### Conversation/session progression

Nodes already own conversation ancestry:

```text
A1 -> A2 -> A3
```

Unrelated conversations advance independently:

```text
A1 -> A2
B1 -> B2
```

Their writes may interleave in the global and Archive clocks without creating ancestry between them.

### Branch/session head revision

`Branch { conversation_id, id, leaf_node_id, canonical }` is a logical branch/session head. Re-appending the same branch identity with a different leaf creates a new immutable revision. Reopen replays revisions in Archive-version order and keeps the latest revision as current.

Historical `branch_at(..., A)` resolves the latest branch revision visible through Archive version `A`.

### Reviving an old conversation

To continue from an old node, create another branch/session head pointing at that node and append new nodes whose `parent_id` descends from it:

```text
A1 -> A2 -> A3
       \
        A4' -> A5'
```

The Archive itself continues forward. No whole-Archive rollback is required and unrelated conversations are unaffected.

### Open

```text
validate container + global clock
    ↓
scan Archive format/content/version metadata
    ↓
validate contiguous Archive versions
    ↓
replay semantic records in Archive-version order
    ↓
latest branch revision wins per branch identity
    ↓
validate references/fragments
```

## State or data ownership

Durable authorities:

```text
CVA header/chunks/global tickets         Container
Archive format marker                    Archive
Archive content objects                  Archive
Archive node/branch/fragment records     Archive
ArchiveRecordVersion metadata            Archive
```

Derived process state:

```text
ContentId -> ChunkRef                  fixed-width content lookup
NodeIndex                              dense Node records + compact hash-to-index slots
BranchIndex                            dense current Branch records + compact hash-to-index slots
FragmentIndex                          dense Fragment records + compact hash-to-index slots
Vec<ArchiveRecordVersion>              dense Archive mutation metadata
```

Node and branch lookups hash `(conversation_id, id)` without materializing composite key strings. Lookup slots contain only record indexes; exact identity is verified against the dense record itself. Content keeps a direct fixed-width hash table because measurement showed an indirect dense lookup would consume more heap for `ContentId -> ChunkRef`.

A historical whole-Archive cut is identified exactly by an Archive version. Full historical-view materialization is not yet exposed as a general API; the retained metadata is sufficient to reconstruct one deterministically.

## Invariants and safety boundaries

- Global and Archive versions are exact integers used for ordering only.
- Archive versions are contiguous within Archive; global versions may have gaps between Archive mutations.
- No Archive-wide semantic parent chain exists.
- Node ancestry never crosses conversation IDs.
- Branch/session revisions do not rewrite older revisions.
- Reviving an old conversation advances a local branch rather than rewinding the Archive.
- Content bodies remain content-addressed and fragments remain branch-neutral ranges.
- Normal Archive writes do not inspect or republish future Memories/Graph/vector state.

See [architectural invariants](invariants.md).

## Code map

| Responsibility | Primary code |
| --- | --- |
| CVA header/chunks | `src/container.rs` |
| Global version clock | `src/container_version.rs` |
| Archive public operations | `src/archive.rs` |
| Archive record-version clocks/history | `src/archive_history.rs` |
| Archive version codec/model | `src/archive_history_codec.rs`, `src/archive_history_model.rs` |
| Archive models/codecs | `src/archive_model.rs`, `src/archive_codec.rs` |
| Reopen/current indexes | `src/archive_store.rs`, `src/archive_lookup.rs`, `src/archive_record_index.rs`, `src/archive_object_index.rs` |
| Fragments | `src/fragment_model.rs`, `src/fragmenter.rs`, `src/fragment_store.rs` |
| History/concurrency semantics tests | `src/history_tests.rs` |
| Corpus smoke | `examples/archive_roundtrip.rs` |
## Tests

Focused tests prove global-clock persistence, independent global/Archive clocks, interleaved unrelated conversations without false ancestry, append-only branch-head revisions, historical branch lookup, old-conversation revival, Archive-version reopen, content dedupe, and fragment invariants.

The prepared corpus smoke continues to verify complete branch/content reconstruction across reopen.

## Related docs

- [Storage format](storage-format.md)
- [Rust API](api.md)
- [Architectural invariants](invariants.md)
- [Versioning and rollback plan](version-history-plan.md)
- [ADR 0003](decisions/0003-layered-version-clocks-and-local-ancestry.md)

## Notes

Whole-CVA restore across several future databases remains a separate design problem. The clocks implemented here deliberately avoid solving it with an every-write global state manifest.