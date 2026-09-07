# ADR 0032: Derived Community lineage and semantic-name continuity

Parent index: [Architectural decisions](INDEX.md)

## Status

Accepted and implemented — 2026-09-07.

## Context

`CommunityId` intentionally identifies exact owner-local membership. A single Memory entering or leaving a Community therefore produces a new ID even when the region is plainly the continuation of the previous one. Exact identity must remain stable and deterministic, but semantic names should not churn for routine membership drift.

Two naming policies are required:

- user-authored names should follow a clear Community continuation rather than disappear after minor membership changes; and
- Dream-generated names should remain stable through small cumulative drift, but become eligible for regeneration after the Community has materially changed from the membership against which the name was generated.

## Decision

Reliquary derives **Community lineage** deterministically from consecutive persisted Community snapshots.

- `CommunityId` remains exact-membership identity and is unchanged by lineage.
- Lineage compares every predecessor/successor pair with non-zero member intersection and records intersection size, predecessor/successor sizes, split/merge participation, and whether the pair is a clear continuation.
- A pair is a clear continuation only when predecessor and successor are each other's unique strongest Jaccard match. Ties are intentionally ambiguous and do not inherit names.
- Split and merge structure is descriptive derived metadata. A dominant split descendant or dominant merge predecessor may still be a continuation when the mutual-best rule is unambiguous; equal or tied splits/merges do not choose a semantic heir.
- Lineage is not separately persisted. Consecutive `CommunitySnapshot` records already contain the complete membership evidence, so `CommunityLineageTransition` is deterministically recomputed from those snapshots without adding another derived record family.
- `community_lineage()` on REL and PHY exposes the transition between the two latest Community generations when one exists.

Semantic-name resolution uses that lineage:

- A direct name on the exact current `CommunityId` wins.
- If there is no direct name, Reliquary walks backward only through clear continuation links.
- A user-authored name inherits through the full clear continuation chain regardless of cumulative membership drift. If lineage becomes ambiguous, inheritance stops rather than choosing arbitrarily.
- A Dream-generated name inherits only while the current Community remains at least **0.750 Jaccard similarity** to the Community membership against which that name was originally generated.
- The Dream comparison is cumulative against the original naming baseline, not against the immediately previous generation. Repeated small changes therefore cannot preserve an obsolete generated label indefinitely.
- When cumulative Dream similarity falls below 0.750, semantic-name resolution returns no inherited Dream name and the Community becomes eligible for normal Dream naming. A newly generated name establishes the new baseline.

`CVACNAM1` schema 3 stores `baseline_community_id` explicitly. Directly persisted records require `baseline_community_id == community_id`; inherited views rewrite only the resolved `community_id` while retaining the original baseline and representative evidence. Historical schema-1 and schema-2 name records remain readable and infer their exact stored `community_id` as the baseline.

## Consequences

Adding one Memory to a large Community normally changes exact `CommunityId` but does not cause Dream naming churn. User labels remain stable across ordinary evolution. Generated names eventually refresh when accumulated change becomes material.

The continuation policy is deliberately conservative. Equal splits, equal merges, and other tied best matches stop inheritance. This may leave a Community temporarily unnamed, but it avoids silently transferring user authority to the wrong descendant.

Lineage remains derived organization, not Graph or Memory authority, and consumes no semantic/global version ticket.

## Rejected alternatives

### Make `CommunityId` continuity-preserving

Rejected. Exact membership identity is useful for deterministic routing, validation, and stale-state detection. Continuity is a separate relation, not identity.

### Compare only adjacent generations for Dream-name retention

Rejected. A sequence of individually tiny changes could otherwise preserve a generated name forever even after the Community had materially diverged from the membership it described.

### Inherit names across every split or merge

Rejected. Ambiguous descendants/ancestors would silently duplicate or misapply user-authored semantic authority.

### Persist a separate lineage record family

Rejected for the current design. Complete historical Community snapshots already provide sufficient deterministic evidence to reconstruct consecutive lineage exactly.

## Verification

`src/community_lineage.rs` protects clear continuation, tied-split ambiguity, user-name inheritance across material drift, and cumulative Dream-name invalidation against the original baseline. Existing Dream naming persistence/reopen tests protect the `CVACNAM1` record path and user-over-Dream precedence.
