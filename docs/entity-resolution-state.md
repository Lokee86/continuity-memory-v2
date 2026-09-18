# Entity mention resolution state

Parent index: [Documentation index](INDEX.md)

## Purpose

Describe the implemented per-mention Entity-resolution state owner, deterministic lifecycle rules, and how the production-capable Entity processor uses that state.

## Overview

Entity resolution is tracked per extracted Memory mention. A Memory may therefore contain any mixture of resolved, unresolved, and rejected Entity mentions.

The persistence/retry machinery is now used by `resolve_entity_mention(...)`, which composes bounded candidate retrieval with zero-candidate Admission or non-empty-candidate V4 identity resolution and then deterministically persists Entity creation/reuse, `Memory -> Entity` association, and mention state. The frozen 41-query zero-Entity bootstrap converges to its complete expected state and survives reopen. This proves the one-shot owner API; it does **not** mean automatic Perception scheduling is wired into the runtime.

## Ownership boundary

Insomnia owns mention extraction through `MemoryRoutingMetadata.entity_mentions`.

Entity resolution never rewrites, combines, or expands those spans. Each resolution record is keyed by the exact Memory ID, field, and byte range of one extracted mention.

Unresolved mentions are **not Entities** and do not receive an `EntityId`.

## States

- `Resolved(EntityId)` — terminal durable association.
- `Rejected` — terminal while the source Memory remains active.
- `Pending` — unresolved and active for reconsideration.
- `Dormant` — unresolved but removed from the active retry set.

`create_new` is not a durable resolution state. After an Entity is actually created, the mention is persisted as `Resolved(new_entity_id)`.

A repeated unresolved result with unchanged candidate/context fingerprints is idempotent. It does not append another record or increase the attempt count.

## Retention

Pending state is deliberately bounded:

- after 30 days without a changed-evidence retry, Pending becomes Dormant;
- after 180 days dormant, the resolution state is tombstoned and removed from active state;
- pending/dormant state is purged immediately when its source Memory is archived or superseded;
- rejected state is also purged when its source Memory becomes inactive;
- resolved associations remain durable.

The original extracted mention stays on the Memory after unresolved state is purged, so later event-driven discovery can reconsider it without retaining an unresolved pseudo-Entity indefinitely.

The REL/PHY container is append-only, so old state records remain historical bytes until a migration/repack rewrites the container. They are not active semantic objects after a tombstone.

## Retry contract

`resolve_entity_mention(...)` calls the retry policy before model resolution. An absent resolution record is eligible. Pending or dormant state is eligible only when the candidate-set fingerprint or relevant-context fingerprint changed. Resolved and rejected mentions are not eligible. Admission context participates in the relevant-context fingerprint, so changed same-surface evidence can make a previously unresolved zero-candidate mention eligible again.

The resolution-state owner itself does not schedule retries or invoke a model; it stores and deterministically governs lifecycle. The Entity processor invokes Admission/V4 when eligible. A future Perception runtime scheduler still owns *when* eligible mentions are submitted to that processor.

## Related docs

- [Perception subsystem plan](perception-subsystem-plan.md)
- [Architecture](architecture.md)
- [Rust API](api.md)
- [Manual: Working with Memories and Entities](manual/memories-and-entities.md)

## Notes

The zero-Entity bootstrap loop is implemented and corpus-proven, but automatic runtime scheduling remains incomplete. Pending/Dormant timing is therefore storage/processor policy rather than a product-level scheduling promise.
