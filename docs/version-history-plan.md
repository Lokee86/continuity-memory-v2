# Versioning, Historical Cuts, and Rollback

Parent index: [Documentation index](INDEX.md)

## Purpose

This document owns the remaining plan for whole-CVA historical recovery now that Archive uses layered clocks and conversation-local ancestry.

## Overview

The implemented model deliberately separates:

```text
CVA global_version       cross-database ordering
Archive archive_version  whole-Archive mutation watermark
Vectors vector_version   vector-generation publication watermark
conversation ancestry    node parent links + branch/session revisions
checkpoint               rebuild acceleration only
```

No integer adjacency is semantic ancestry.

## Current status

Implemented for Archive:

- container-issued monotonically increasing global `u64` tickets;
- contiguous Archive-local `u64` versions;
- one `ArchiveRecordVersion { global_version, archive_version, record }` per semantic Archive mutation;
- node parent links scoped to a conversation;
- repeated immutable branch/session-head revisions;
- historical branch-head lookup at an Archive watermark;
- continuation from an old node through a new local branch;
- no Archive-wide parent-linked publication chain.

The CVA now also contains immutable packed-vector matrices, Archive-Vector bindings, immutable Embedding Profiles, and a mutable VectorGenerationStore. Vector-generation publication has its own dense `vector_version` and consumes CVA-global tickets; the latest visible generation per profile is current. This is the second concrete mutable semantic timeline. Generic whole-CVA historical materialization, restore-and-continue activation, retention/vacuum, and concurrent multi-writer publication remain unimplemented.

## Expected ownership or ownership boundary

The container owns only the CVA-global ordering ticket. Each database may own a local monotonic watermark if useful for its own snapshots/history.

Archive owns `archive_version`. VectorGenerationStore owns `vector_version`. Neither local watermark is a generalized dependency identity. A generation may record a concrete `source_archive_version` because it is derived from a specific Archive cut; that explicit source fact does not make Archive's clock globally semantic.

Conversation/session ancestry remains Archive domain data, not a clock concern.

## Planned behavior

### Global ordering

Current semantic records can interleave:

```text
G100 Archive A700
G101 Vectors V20
G102 Archive A701
G103 Vectors V21
```

The global number answers “when did this mutation enter the CVA ordering?” It does not answer “what is its semantic parent?”

### Archive cuts

An Archive watermark identifies one whole-Archive historical cut:

```text
A700 = replay Archive semantic mutations 1..700
```

Because logical branch/session heads are revisions, replay chooses the newest visible revision for each branch identity while retaining all immutable nodes/fragments visible through the cut.

A future `ArchiveView(A700)` can materialize this without any Archive-wide parent chain.

### Conversation/session branching

Conversation history is independent:

```text
A1 -> A2 -> A3
       \
        A4' -> A5'
```

The Archive watermark continues forward while this branch is created. Other conversations are unaffected.

### Whole-CVA historical point

A global point `G=N` can now conceptually resolve Archive to the latest `archive_version` visible at or before `N` and Vector Generations to the latest `vector_version` visible at or before `N`. Within that vector cut, the newest visible generation for each profile is current.

The public API does not yet materialize that combined read-only view. **Continuing from a whole-CVA historical point** remains harder because future writes must not silently reincorporate abandoned later state. With two mutable semantic domains now implemented, that ownership problem is concrete enough to design next rather than speculate through a generalized state root.

### Checkpoints

An Archive checkpoint may say:

```text
checkpoint covers through A85000
```

It accelerates open/replay. It is disposable/rebuildable and is not a rollback root.

## Implementation sequence

1. Compact Archive lookup representation keyed to an Archive watermark. **Implemented.**
2. Add immutable packed-vector backing storage without inventing semantic publication. **Implemented.**
3. Add immutable Archive-Vector row bindings without embedding-profile or publication semantics. **Implemented.**
4. Add immutable embedding profiles plus vector-generation publication with a dense local watermark. **Implemented.**
5. Prove global ordering across Archive and Vector Generations without semantic coupling. **Implemented.**
6. Define read-only whole-CVA point materialization from global ordering plus the two local cuts.
7. Re-evaluate optional Archive checkpoint load + replay tail from larger cold-open measurements.
8. Only then define crash-safe restore-and-continue plus retention/vacuum semantics.

## Acceptance criteria

- Unrelated conversations can advance without a shared semantic head.
- Every Archive semantic mutation has exact global and Archive ordering.
- Every published vector generation has exact global and vector-local ordering.
- Archive and vector-generation cuts are independently reconstructible.
- No CVA-global version is claimed by more than one semantic mutation across those stores.
- Local conversation revival does not rewind unrelated Archive state.
- Future store-local clocks do not become cross-store dependency identities.
- Whole-CVA restore, when implemented, does not require every normal write to publish a global state manifest.

## Open decisions

- Generic historical `ArchiveView` representation/API.
- Concurrent reservation of physical append positions and global versions.
- Read-only whole-CVA view representation/API across Archive and Vector Generations.
- Whole-CVA restore-and-continue timeline identity now that multiple mutable stores exist.
- Retention/pinning/vacuum policy for abandoned histories.
- User-facing terminology for archive cuts, sessions, and timelines.

## Related docs

- [Architecture](architecture.md)
- [Storage format](storage-format.md)
- [ADR 0003](decisions/0003-layered-version-clocks-and-local-ancestry.md)
- [ADR 0005](decisions/0005-cva-composition-and-packed-vector-objects.md)
- [ADR 0006](decisions/0006-archive-vector-row-bindings.md)
- [ADR 0007](decisions/0007-embedding-profiles-and-vector-generations.md)
- [Superseded ADR 0002](decisions/0002-branching-publication-history.md)
- [Roadmap](roadmap.md)

## Notes

The key correction is scope: immutable record history and conversation/session ancestry are local facts; the Archive as a whole is a database observed at integer watermarks, not one giant parent-linked record.