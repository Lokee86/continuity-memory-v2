# Versioning, Historical Cuts, and Rollback

Parent index: [Documentation index](INDEX.md)

## Purpose

This document owns the remaining plan for whole-CVA historical recovery now that Archive uses layered clocks and conversation-local ancestry.

## Overview

The implemented model deliberately separates:

```text
CVA global_version      cross-database ordering
Archive archive_version whole-Archive mutation watermark
conversation ancestry   node parent links + branch/session revisions
checkpoint              rebuild acceleration only
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

The CVA also contains immutable packed-vector backing objects, but those objects are not yet a second mutable semantic timeline: raw matrix creation consumes no global or Archive semantic version. Not implemented: generic full historical `ArchiveView`, embedding-profile/Archive-Vector semantic generations, multi-database whole-CVA restore activation, retention/vacuum, and concurrent multi-writer publication.

## Expected ownership or ownership boundary

The container owns only the CVA-global ordering ticket. Each database may own a local monotonic watermark if useful for its own snapshots/history.

Archive owns the meaning of `archive_version`; other databases must not depend semantically on it.

Conversation/session ancestry is Archive domain data, not a container or Archive-clock concern.

## Planned behavior

### Global ordering

Future records may interleave:

```text
G100 Archive A700
G101 Memories M220
G102 Archive A701
G103 Graph G55
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

A global point `G=N` can eventually resolve each concrete database to the latest local watermark/record visible at or before `N`.

That is enough for read-only point-in-time inspection. **Continuing from a whole-CVA historical point** is harder because future writes must not silently reincorporate abandoned later state. We will not invent that timeline mechanism until at least two mutable databases exist and its ownership is concrete.

### Checkpoints

An Archive checkpoint may say:

```text
checkpoint covers through A85000
```

It accelerates open/replay. It is disposable/rebuildable and is not a rollback root.

## Implementation sequence

1. Compact Archive lookup representation keyed to an Archive watermark. **Implemented.**
2. Add immutable packed-vector backing storage without inventing semantic publication. **Implemented.**
3. Add embedding profiles, then Archive Vectors as the second mutable semantic store with its own local sequencing only if needed.
4. Prove global ordering across Archive and the first vector semantic store without semantic coupling.
5. Re-evaluate optional Archive checkpoint load + replay tail from larger cold-open measurements.
6. Define read-only whole-CVA point materialization.
7. Only then define crash-safe whole-CVA restore-and-continue timeline semantics.
8. Define retention/vacuum reachability for old local branches/timelines.

## Acceptance criteria

- Unrelated conversations can advance without a shared semantic head.
- Every Archive semantic mutation has exact global and Archive ordering.
- Archive cuts are deterministically reconstructible.
- Local conversation revival does not rewind unrelated Archive state.
- Future store-local clocks do not become cross-store dependency identities.
- Whole-CVA restore, when implemented, does not require every normal write to publish a global state manifest.

## Open decisions

- Generic historical `ArchiveView` representation/API.
- Concurrent reservation of physical append positions and global versions.
- Whether Archive Vector semantic generations need a local counter analogous to Archive's; immutable packed-vector backing objects do not.
- Whole-CVA restore-and-continue timeline identity after multiple stores exist.
- Retention/pinning/vacuum policy for abandoned histories.
- User-facing terminology for archive cuts, sessions, and timelines.

## Related docs

- [Architecture](architecture.md)
- [Storage format](storage-format.md)
- [ADR 0003](decisions/0003-layered-version-clocks-and-local-ancestry.md)
- [ADR 0005](decisions/0005-cva-composition-and-packed-vector-objects.md)
- [Superseded ADR 0002](decisions/0002-branching-publication-history.md)
- [Roadmap](roadmap.md)

## Notes

The key correction is scope: immutable record history and conversation/session ancestry are local facts; the Archive as a whole is a database observed at integer watermarks, not one giant parent-linked record.