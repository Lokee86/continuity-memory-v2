# ADR 0025: Owner-local derived Graph communities

Parent index: [Architectural decisions](INDEX.md)

## Status

Accepted and implemented — 2026-08-27.

## Context

Dream now publishes durable semantic relationships into the owner-local Memory Graph for both Reliquary and Phylactery. That Graph is authoritative relationship state, but a large connected Graph still needs deterministic structural regions that later traversal and presentation can use without asking a model to classify or name every region.

Community detection must not become a second relationship authority. It must not rewrite Dream edges, create semantic supernodes, consume semantic version tickets, or introduce cross-owner Graph state. Human-facing labels are also presentation metadata rather than part of community identity.

## Decision

Reliquary persists owner-local **derived community snapshots** over the current Graph.

- A snapshot records its own monotonic `generation` plus the exact `derived_graph_version` it was computed from. Community generation is derived-state bookkeeping only; it is not a fifth semantic clock and consumes no `CVAVERS1` ticket.
- A snapshot is current only while its `derived_graph_version` equals the owner's current `graph_version`.
- The first implementation is an explicit full-Graph baseline. Every active same-file Graph relationship is projected to an undirected structural pair for clustering. Multiple active relationship identities between the same unordered Memory pair collapse to one unweighted structural edge.
- Baseline clustering uses Leiden modularity with resolution `1.0`, a fixed seed, and deterministic sequential execution. The algorithm/configuration version is persisted with each snapshot.
- Leiden never mutates Graph authority. The original oriented relationship vocabulary and histories remain entirely Graph-owned.
- Every current Graph node belongs to exactly one community in a current snapshot. REL and PHY are clustered independently; no community may contain nodes from another durable owner.
- Baseline `CommunityId` is deterministic exact-membership identity: SHA-256 over an explicit domain separator, the durable owner UUID, and the sorted member `MemoryId` values. The baseline makes no promise that an ID survives a membership change.
- Human-facing community names are outside the Reliquary identity contract. Warlock may attach or edit display names without changing membership, Graph relationships, or traversal semantics.
- A real divergent semantic reconciliation repack may discard community snapshots and rebuild them from the merged Graph. A semantic no-op reconciliation preserves the canonical file byte-for-byte, including any current snapshot.

The Leiden implementation is isolated behind a private adapter. Reliquary owns only the projection policy and persisted derived snapshot semantics. If the generic graph layer later exposes a suitable Leiden primitive, the adapter can move behind that boundary without changing the persisted community contract.

## Consequences

The memory Graph gains deterministic, reopenable structural regions without adding model inference or another semantic authority. Callers can test snapshot freshness cheaply against `graph_version`, and unchanged refresh is an idempotent no-op.

The baseline deliberately pays the cost of reclustering the full owner Graph after a change. Community identity also changes when exact membership changes. Incremental affected-region scan-and-merge, continuity-preserving split/merge lineage, and community-aware traversal are separate later work and must be designed from measured behavior rather than hidden inside the baseline.

## Rejected alternatives

### Rewrite or augment Graph during clustering

Rejected. Leiden detects communities implied by existing topology; it does not establish semantic Memory relationships. Persisting synthetic semantic edges or supernodes would create competing Graph authority.

### Use an LLM to define communities

Rejected for the structural layer. Community membership is deterministic Graph-derived state. Optional human-readable naming belongs to presentation metadata and may be model-assisted later without becoming clustering authority.

### Implement an approximate home-grown Leiden/Louvain routine

Rejected. The baseline uses an existing Leiden implementation behind a narrow adapter rather than introducing an algorithm that merely resembles Leiden while carrying the same name.

### Promise stable IDs across membership changes now

Rejected. Split/merge continuity requires explicit reconciliation semantics. Exact-membership IDs are sufficient for the full-rebuild baseline and avoid pretending that a changed cluster has an already-defined durable identity.

## Verification

`src/community_tests.rs` verifies dense-region detection, deterministic persistence/reopen, unchanged-refresh idempotency, stale-on-Graph-change behavior, independent Phylactery operation, and rejection of community publication for legacy files without durable owner identity.
