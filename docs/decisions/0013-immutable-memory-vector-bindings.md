# ADR 0013: Immutable Memory Vector bindings

## Status

Accepted — 2026-08-15.

## Context

Working Memory revisions may change classification, lifecycle, scope, provenance-adjacent metadata, and later relationship state without changing the semantic memory text itself. Regenerating an embedding merely because a Memory revision advanced would create unnecessary work and would incorrectly make mutable metadata part of vector identity.

The CVA already has an immutable `MemoryBodyId` derived from Memory title + content, a shared immutable `PackedVectorStore`, and endpoint-independent `CompatibilityProfileId` contracts.

## Decision

Memory semantic title/content is immutable for a stable `MemoryId`. A later revision that attempts to change the `MemoryBodyId` is rejected. Semantic correction or replacement creates another Memory rather than mutating the existing semantic body.

Memory Vectors are immutable derived bindings keyed by the pair:

```text
(CompatibilityProfileId, MemoryBodyId)
```

A Memory Vector set contains one compatibility profile, one shared `PackedVectorId`, and an ordered list of `MemoryBodyId`s whose row ordinals map directly to the packed matrix rows. Exactly one binding may exist for a profile/body pair.

Memory Vectors consume no CVA-global semantic version and own no local generation/revision clock. Metadata-only Memory revisions cannot invalidate, refresh, or regenerate an embedding. If another compatibility profile is established, the same Memory body may receive a second immutable vector under that profile.

The high-level builder verifies the selected endpoint against the compatibility profile and embeds only current Memory bodies that do not already have a binding for that profile. The canonical current embedding input is Memory title, two newlines, then Memory content.

## Consequences

- Memory authority remains independent from embedding availability.
- There is no per-Memory vector update machinery.
- Dream or other metadata processing can revise Memory records without causing embedding work.
- Profile changes can create additional immutable vectors without rewriting earlier vectors.
- Packed numeric storage is shared physically with Archive vectors, while Memory Vector row identity is owned by a separate index/store.
- Development CVAs created before the `CVAMVFM1` format marker are intentionally rejected; migration scaffolding is not added during the rebuild.

## Rejected alternatives

### Bind vectors to Memory revision

Rejected. Revision identity includes mutable metadata that does not alter semantic embedding content and would cause needless re-embedding.

### Mutate one vector when Memory metadata changes

Rejected. Metadata is outside the semantic vector payload, and packed vectors are immutable backing objects.

### Store vector bytes directly in Memory records

Rejected. It mixes authoritative Memory state with derived vector representation and prevents physical vector storage from remaining shared and representation-specific.

## Verification

Focused tests prove one embedding per Memory body/profile across metadata-only revisions, rejection of semantic body mutation in place, distinct bindings for distinct compatibility profiles, and full reopen reconstruction of Memory Vector bindings.

## References

- [Architecture](../architecture.md)
- [Storage format](../storage-format.md)
- [ADR 0001](0001-purpose-built-database-ownership.md)
- [ADR 0007](0007-compatibility-profiles-and-vector-generations.md)
- [ADR 0012](0012-deterministic-episodes-and-insomnia-memory-authority.md)
