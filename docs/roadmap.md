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
10. Compact Archive-owned derived indexes without composite string keys; current measurements are tracked in `development.md`.
11. Single-pass Container/Archive reopen reconstruction.
12. Lodestone-derived generic packed-vector rows plus an immutable content-addressed packed-vector store inside the CVA.
13. Immutable Archive-Vector sets that bind packed rows to ordered Archive `FragmentId`s with exact cross-store validation in the same physical reopen scan.
14. Immutable endpoint-independent compatibility profiles with Query/Document reference probes, tolerant cosine verification, and deterministic simulated endpoints.
15. Vector-generation semantic publication with dense local `vector_version`, CVA-global ordering, per-profile current generations, Archive coverage validation, and inert incomplete publications.
16. Exact semantic retrieval through compatibility verification, Query-mode embedding, current-generation resolution, exact cosine scan, and row-to-FragmentId mapping.
17. Original default lexical/hybrid retrieval with 30-candidate fusion, `0.45/0.55` channel weighting, overlap diversification, and 10 final results.
18. Purpose-built replaceable local configuration container with fragment/retrieval objects, deterministic framing, unknown-object preservation, and atomic whole-file replacement.
19. Expandable model-switchboard routing with `General`/`Embedding` capabilities, `openai-codex`/`openai-ready` providers, explicit auth kinds, and persisted `models.general` / `models.embedding` objects.
20. Self-generated 256-bit master key with a temporary JSON-backed key-store seam for later encrypted credential objects.
21. AES-256-GCM credential objects with stable credential IDs, model-route references, wrong-key/tamper rejection, and switchboard auth-header attachment.

## Expected ownership or ownership boundary

`Cva` owns physical composition and the single Container handle. Archive owns source-history semantics and `archive_version`. Packed vectors own immutable numeric matrices; Archive Vectors own immutable row-to-fragment bindings; Compatibility Profiles own immutable vector-space compatibility contracts. Vector Generations own profile/population activation and the independent dense `vector_version`. Archive and Vector Generations interleave only through CVA-global ordering.

## Planned behavior

Near-term priorities:

1. Add direct provider transport: `openai-ready` API-key HTTP first, then `openai-codex` ChatGPT device-code acquisition/token refresh; replace temporary JSON key persistence with an OS credential-store implementation before production.
2. Add bounded ancestry-aware Archive packing from measured retrieval/access patterns, with compression at pack level and shared branch ancestry stored once.
3. Measure pack size/compression tradeoffs plus repeated cold-open/search scaling on substantially larger and realistic-dimension vector-bearing Archives; add persistent Archive checkpoints or ANN search only if measurements justify them.
4. Define read-only whole-CVA historical materialization now that Archive and Vector Generations provide two concrete mutable semantic domains; defer restore-and-continue branching until that model is proven.
5. Add Memories, Memory Vectors, then Graph as separate owners.
6. Build the shared long-lived Continuity runtime, move endpoint compatibility verification to a cached runtime capability boundary, and reconnect Insomnia/Dream, then Ego.

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
- Calibration of compatibility policy v1 against representative real routed/local embedding endpoints; switchboard routing is implemented but only simulated embedding execution is wired.
- Explicit vector-generation retirement/deactivation and retention policy.
- Quantization metadata and alternate packed representations for published generations.
- Whole-CVA historical materialization and restore/timeline representation now that two mutable semantic domains exist.
- Retention/vacuum semantics for abandoned conversation/session branches.

## Related docs

- [Architecture](architecture.md)
- [Current limitations](current-limitations.md)
- [Versioning and rollback plan](version-history-plan.md)
- [ADR 0003](decisions/0003-layered-version-clocks-and-local-ancestry.md)
- [ADR 0009](decisions/0009-expandable-model-switchboard.md)
- [ADR 0010](decisions/0010-encrypted-credential-objects.md)

## Notes

Sequence can change with measurements; ownership boundaries should not.