# ADR 0024: Owner-local Dream processing

Status: Accepted and implemented, 2026-08-27

Owners: Dream candidate discovery, Graph publication, Memory lifecycle projection, ReliquaryRuntimeHost

Extends ADR 0020's REL/PHY file split, ADR 0022's durable owner identity, and ADR 0023's durable Insomnia owner routing.

## Context

Dream was originally implemented against `Cva` because REL Memories could recover authoritative chronology through Archive/Episode provenance. Phylactery deliberately does not retain those source pointers. Routed User Memories now persist `source_time_ns` before REL-local provenance is stripped, so PHY has the chronology Dream needs without depending on an originating Project Archive.

The remaining question is whether Dream should compare REL and PHY as one graph, or operate independently inside each durable owner.

## Decision

Dream is **owner-local**.

A Dream pass runs against exactly one owner at a time:

```text
Dream(Project REL)
Dream(User PHY)
```

Candidate discovery, classifier/verifier context, Graph reads/writes, duplicate-chain maintenance, supersession, and lifecycle projection never cross that owner boundary. The shared implementation operates on the common Container/Memory/Graph/Memory-vector/Packed-vector mechanics; `Cva` and `Phylactery` provide owner-specific wrappers and chronology resolution.

REL chronology resolves persisted `source_time_ns` first and may fall back to validated Archive/Episode provenance for legacy Memories. PHY chronology is only persisted `source_time_ns`. `created_at_ns` is bookkeeping and is never substituted for unknown semantic chronology.

Duplicate ordering requires authoritative source chronology. If a participating Memory has no source timestamp, duplicate-chain publication fails closed rather than fabricating order.

PHY lifecycle uses the owner-independent Dream rules: duplicate archival, supersession projection, direct-authority canonical promotion, inherited canonical status through unambiguous supersession, and ordinary extracted-to-knowledge completion. REL's corroboration promotion remains REL-specific because its proof currently depends on independent source authority anchors that PHY intentionally does not retain. No replacement PHY corroboration heuristic is invented.

`DreamProcessor` exposes separate REL and PHY processing methods. In addition to the single-Memory path, bounded population processing uses deterministic same-owner frontiers. Candidate sets for one frontier are selected before inference begins; candidate-pair work is round-robin interleaved across those Memories and consumed by one globally bounded worker pool. Completed inference is then published in source order before the next frontier is selected. This removes whole-Memory inference barriers without multiplying the configured provider route or credential. Classification/verification failure publishes nothing for that source Memory; structural candidate/publication/lifecycle failures abort the batch.

`InteractionRuntime::process_background_work` uses that frontier primitive, with frontier size and aggregate inference concurrency as explicit runtime configuration. The current defaults are six Memories per frontier and twelve concurrent pair evaluations. `ReliquaryRuntimeHost` derives independent extracted-Memory backlogs from the active REL and optional attached PHY after each owner has a compatible Memory-vector binding. Model inference occurs outside REL/PHY/runtime locks. Before publication, the host reacquires only the target owner and revalidates the snapshotted Graph version plus all participating Memory revisions; stale inference is discarded and retried from fresh state.

Dream still owns no durable queue. Successful lifecycle projection removes the Memory from the derived `extracted` backlog. Classification/verification failures leave it extracted and eligible for bounded retry.

## Explicit non-goals

This decision does **not** add:

- REL-to-PHY or PHY-to-REL candidate comparison;
- persisted cross-file Graph edges;
- cross-owner duplicate or supersession chains;
- lifecycle mutation in one owner because of state in another owner;
- canonicalization or provenance transfer across owners.

Cross-owner semantic composition is deliberately outside Dream. If a host later needs Project and User state in one execution context, retrieval/Ego queries those owners independently and composes the returned state without turning Dream into a federation layer. `MemoryRef` remains useful for narrow cross-file receipts/references, not for Dream Graph semantics.

## Consequences

- PHY Dream no longer requires Archive/history or a live originating REL.
- REL and PHY can be processed concurrently without entangling their Graph/version domains.
- Source-independent User Memories retain semantic chronology without retaining project-local provenance.
- Existing same-file Graph invariants remain unchanged.
- Corroboration semantics remain conservative rather than being weakened to fit PHY.
- Organization and Connection RELs inherit the same same-owner Dream mechanics because they use the Reliquary owner model; routing policy into those scopes remains separate work.

## Validation

The frozen 11-Episode Insomnia fixture was routed through production Insomnia into separate owners on 2026-08-27: 41 Project Memories and 8 User Memories, with all 49 vectorized in their respective files. Same-owner Dream drained the eight PHY Memories with zero inference failures to five knowledge and three canonical Memories, persisting 16 PHY-local active relations. The 41-Memory REL run produced 121 REL-local active relations; one malformed classifier response left its source extracted as designed, and the next retry completed that source with no change to the owner boundary.
