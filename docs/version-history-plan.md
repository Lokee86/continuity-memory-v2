# Versioning, Historical Cuts, and Rollback

Parent index: [Documentation index](INDEX.md)

## Purpose

This document owns future work for whole-CVA historical views, restore-and-continue, retention, and rollback. Current versioning behavior is documented in [Architecture](architecture.md), [Storage format](storage-format.md), and [Architectural invariants](invariants.md).

## Overview

Future historical recovery composes owner-local history at explicit cuts rather than inventing a CVA-wide semantic ancestry chain. Read-only historical views, retained-line identity, restore-and-continue, checkpoint acceleration, and physical reclamation are separate stages with explicit recovery semantics.

## Governing constraints

Future historical recovery must preserve these architectural constraints:

- semantic owners retain independent local history/watermark semantics;
- CVA-global ordering remains ordering rather than semantic ancestry;
- conversation/session ancestry remains source-domain data;
- explicit cross-owner source watermarks may be recorded when semantically meaningful, but no generalized dependency identity is introduced;
- checkpoints remain disposable reconstruction acceleration, never rollback authority;
- ordinary writes must not publish a CVA-wide state manifest;
- restore-and-continue must not silently reincorporate state that the selected historical line intentionally abandoned.

## Planned behavior

### 1. Owner-local historical views

Define read-only historical-view APIs for each mutable semantic owner that needs them. Views must resolve owner state at an explicit local watermark without mutating current state or introducing parent links between whole-database publications.

### 2. Whole-CVA read-only historical point

Define a resolver for a CVA-global ordering point that selects the latest visible state of each mutable semantic owner at or before that point.

The resulting view must:

- be read-only;
- preserve owner-local semantics;
- expose which local cut was selected for each owner;
- reject missing/corrupt cross-owner references at that historical point;
- avoid fabricating a generalized CVA root object.

### 3. Pins, retention, and timeline identity

Before destructive reclamation or restore-and-continue exists, define durable identities for historical lines that must remain reachable.

Required semantics include:

- user/system pins;
- current-line identity;
- abandoned-but-retained lines;
- minimum retained source state required by surviving cross-owner references;
- generation/branch/session retention interaction.

### 4. Crash-safe restore-and-continue

Define how a user selects a historical whole-CVA point and continues from it without rewriting old immutable history.

The new line must have explicit identity, and later writes must attach to that selected line rather than implicitly seeing superseded future state.

This design must remain compatible with purpose-built semantic owners and must not force unrelated databases to republish heads on every normal write.

### 5. Checkpoint acceleration

Add owner-specific or composition-level derived checkpoints only after cold-open/replay measurements justify them.

A checkpoint may accelerate reconstruction through a documented watermark but must be fully disposable and reproducible from authoritative records.

### 6. Reachability, compaction, and vacuum

After timeline/pin semantics exist, define physical reclamation:

- compute authoritative reachability from retained semantic lines;
- retain backing objects required by any surviving owner/reference;
- reclaim abandoned chunks safely;
- optionally rewrite/compact the physical file without changing semantic identities;
- verify equivalence before replacing the source CVA.

### 7. Concurrent publication mechanics

Define physical append-position and global-version reservation for multiple live writers only after the long-lived runtime demonstrates a concrete need. Concurrency mechanics must not introduce a shared semantic head.

## Implementation sequence

```text
owner-local historical views
    ↓
whole-CVA read-only point
    ↓
pins + retention/timeline identity
    ↓
restore-and-continue
    ↓
checkpoint acceleration as measured
    ↓
reachability + compaction/vacuum
```

Concurrent publication mechanics can be developed independently once a real runtime workload requires them.

## Acceptance criteria

- A historical read never mutates current semantic state.
- A whole-CVA point reports explicit per-owner cuts.
- No global ordering integer becomes semantic ancestry.
- Restore-and-continue produces an explicit new retained line rather than rewriting old records.
- Abandoned later state cannot reappear implicitly after restore.
- Normal writes remain owner-local and do not publish a global state manifest.
- Checkpoints are removable without changing historical meaning.
- Vacuum never reclaims an object reachable from any retained line.
- Crash/reopen during restore or compaction fails safely to either the old valid file/line or the new valid file/line.

## Open decisions

- Historical-view API shape and terminology.
- Whole-CVA point representation.
- Timeline/line identity representation.
- Pinning ownership and user-facing controls.
- Cross-owner reachability calculation without a generalized dependency engine.
- Restore activation record format and crash boundary.
- Retention defaults for branches, Memories, vector generations, files, and future semantic owners.
- Checkpoint representation/cadence.
- Compaction replacement strategy and verification.
- Concurrent append/version reservation mechanics.

## Related docs

- [Architecture](architecture.md)
- [Storage format](storage-format.md)
- [Architectural invariants](invariants.md)
- [ADR 0003](decisions/0003-layered-version-clocks-and-local-ancestry.md)
- [ADR 0005](decisions/0005-cva-composition-and-packed-vector-objects.md)
- [ADR 0006](decisions/0006-archive-vector-row-bindings.md)
- [ADR 0007](decisions/0007-compatibility-profiles-and-vector-generations.md)
- [Roadmap](roadmap.md)

## Notes

This is a future-only planning document. Current historical/version behavior belongs in the current architecture and storage references.
