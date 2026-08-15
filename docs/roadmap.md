# Roadmap

Parent index: [Documentation index](INDEX.md)

## Purpose

This document owns the cross-cutting implementation sequence for the clean Continuity rebuild.

## Overview

Concrete storage owners come first. Shared mechanics are generalized only after at least two concrete owners prove the same requirement.

## Current status

Completed bootstrap slices:

1. CVA create/open/header and opaque chunk storage.
2. Container-global monotonic `u64` ordering.
3. Archive content-addressed bodies and branch-aware node graph.
4. Durable range-only fragments with 8-turn / 2-overlap policy.
5. Archive-local contiguous `u64` mutation ordering.
6. Dual `(global_version, archive_version)` metadata per semantic Archive mutation.
7. Append-only branch/session-head revisions with historical lookup.
8. Old-conversation revival through conversation-local branching, not Archive rollback.
9. Reopen validation and prepared-corpus round trip.
10. Compact Archive-owned derived indexes without composite string keys; current prepared-corpus retained open heap is `752,907` bytes.
11. Single-pass Container/Archive reopen reconstruction.
12. Lodestone-derived generic packed-vector rows plus an immutable content-addressed packed-vector store inside the CVA.
13. Immutable Archive-Vector sets that bind packed rows to ordered Archive `FragmentId`s with exact cross-store validation in the same physical reopen scan.

## Expected ownership or ownership boundary

`Cva` owns physical composition and the single Container handle. Archive owns Archive-local clocks and source-history semantics. Packed vectors own immutable numeric matrices; Archive Vectors own immutable row-to-fragment bindings. Neither backing layer consumes semantic clocks. Embedding-profile/generation, Memories, Graph, and later semantic owners receive local sequencing only if their mutation semantics require it.

## Planned behavior

Near-term priorities:

1. Implement durable embedding profiles as embedding-space identity, independent of Archive row bindings.
2. Implement vector-generation publication that associates one profile with one Archive-Vector set plus coverage/activation metadata.
3. Recover simple lexical + exact similarity retrieval before considering specialized indexes.
4. Add bounded ancestry-aware Archive packing from measured retrieval/access patterns, with compression applied at pack level rather than as a prerequisite per-record feature; shared branch ancestry must remain stored once.
5. Measure pack size/compression tradeoffs plus repeated cold-open scaling on substantially larger Archives; add persistent Archive checkpointing only if those measurements justify it.
6. Add Memories, Memory Vectors, then Graph as separate owners.
7. Define rare whole-CVA rollback/timeline activation across those stores without adding an every-write global state manifest.
8. Build the shared long-lived Continuity runtime and reconnect Insomnia/Dream, then Ego.

## Implementation sequence

For each new store:

```text
define authority + records
    ↓
define local mutation/revision semantics
    ↓
attach global ordering only as needed
    ↓
prove reopen/recovery
    ↓
measure simple implementation
    ↓
add acceleration
```

## Acceptance criteria

A storage slice is complete only when ownership, persistent format, failure/recovery, focused tests, documentation coverage, and derived-vs-authoritative state are explicit.

## Open decisions

- Archive checkpoint representation, cadence, and retention.
- Exact pack target size, Archive record grouping, and compression codec; ADR 0004 fixes the bounded ancestry-aware shape but leaves these measurement-driven.
- Actual concurrent file append/version reservation mechanics.
- Embedding-profile and vector-generation publication semantics, including whether the first active vector-generation owner needs a dense local watermark.
- Whole-CVA restore/timeline representation after at least two mutable semantic domains exist.
- Retention/vacuum semantics for abandoned conversation/session branches.

## Related docs

- [Architecture](architecture.md)
- [Current limitations](current-limitations.md)
- [Versioning and rollback plan](version-history-plan.md)
- [ADR 0003](decisions/0003-layered-version-clocks-and-local-ancestry.md)

## Notes

Sequence can change with measurements; ownership boundaries should not.