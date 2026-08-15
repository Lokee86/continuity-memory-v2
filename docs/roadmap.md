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
10. Compact Archive-owned derived indexes without composite string keys; prepared-corpus retained open heap is `752,897` bytes.
11. Single-pass Container/Archive reopen reconstruction; current `2.2 MB` prepared corpus measures `26.102 ms` warm median / `26.917 ms` p90 and `26.303 ms` in the verified single cold-cache sample.

## Expected ownership or ownership boundary

Archive owns Archive-local clocks, record visibility, node ancestry, branch/session revisions, and future Archive checkpoints. The container owns only global physical ordering. Future Memories, Graph, and vector databases receive their own concrete local sequencing only if needed.

## Planned behavior

Near-term priorities:

1. Implement Archive Vectors as the second concrete database and validate whether the dual-clock mechanic is genuinely reusable.
2. Recover simple lexical + exact-cosine retrieval before considering specialized indexes.
3. Add bounded ancestry-aware Archive packing from measured retrieval/access patterns, with compression applied at pack level rather than as a prerequisite per-record feature; shared branch ancestry must remain stored once.
4. Measure pack size/compression tradeoffs plus repeated cold-open scaling on substantially larger Archives; add persistent Archive checkpointing only if those measurements justify it, and keep any checkpoint derived state keyed by an Archive watermark.
5. Add Memories, Memory Vectors, then Graph as separate owners.
6. Define rare whole-CVA rollback/timeline activation across those stores without adding an every-write global state manifest.
7. Build the shared long-lived Continuity runtime and reconnect Insomnia/Dream, then Ego.

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
- Whole-CVA restore/timeline representation after at least two mutable domains exist.
- Retention/vacuum semantics for abandoned conversation/session branches.

## Related docs

- [Architecture](architecture.md)
- [Current limitations](current-limitations.md)
- [Versioning and rollback plan](version-history-plan.md)
- [ADR 0003](decisions/0003-layered-version-clocks-and-local-ancestry.md)

## Notes

Sequence can change with measurements; ownership boundaries should not.