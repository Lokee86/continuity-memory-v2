# Entity mention resolution state

Parent index: [Documentation index](INDEX.md)

## Purpose

Describe the implemented per-mention Entity-resolution state owner, deterministic lifecycle rules, and how the production-capable Entity processor uses that state.

## Overview

Entity resolution is tracked per extracted Memory mention. A Memory may therefore contain any mixture of resolved, unresolved, and rejected Entity mentions.

The persistence/retry machinery is used by `resolve_entity_mention(...)`, which composes bounded candidate retrieval with zero-candidate Admission or non-empty-candidate V4 identity resolution and then deterministically persists Entity creation/reuse, `Memory -> Entity` association, and mention state. The frozen 41-query zero-Entity bootstrap converges to its complete expected state and survives reopen. The long-lived runtime host now schedules this Entity pass automatically after Dream for both REL and attached PHY.

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

A repeated unresolved result with unchanged candidate/context fingerprints is idempotent. The runtime may receive a wake event, but it rebuilds the candidate/evidence state first and makes no model call, appends no record, and does not increase the attempt count when both hashes are unchanged. The persisted field names remain `candidate_set_fingerprint` and `context_fingerprint`; semantically they are hashes of candidate identity/state and the bounded resolution evidence supplied to Admission/V4.

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

`resolve_entity_mention(...)` calls the retry policy before model resolution. An absent resolution record is eligible. Pending or dormant state is eligible only when the candidate-set fingerprint or relevant-context fingerprint changed. Resolved and rejected mentions are not eligible. Admission context participates in the relevant-context fingerprint, so changed same-surface evidence can make a previously unresolved zero-candidate mention eligible again. Candidate-attached Memory evidence likewise participates, so a new association/evidence Memory on a previously considered candidate can make that mention eligible.

The resolution-state owner itself does not invoke a model; it stores and deterministically governs lifecycle. The Perception runtime scheduler owns *when* mention keys are reconsidered. Its wake sources are bounded deterministic events: post-Dream source/affected Memories, same-surface mention evidence, and changes to evidence associated with previously considered candidate Entities. The scheduler then prepares a fresh snapshot, invokes Admission/V4 only when the retry hashes changed, and commits only if the durable owner identity and Memory/Entity/Graph/resolution versions still match.

## Related docs

- [Perception subsystem plan](perception-subsystem-plan.md)
- [Architecture](architecture.md)
- [Rust API](api.md)
- [Manual: Working with Memories and Entities](manual/memories-and-entities.md)

## Notes

The zero-Entity bootstrap loop and automatic runtime scheduling are implemented. Pending→Dormant and Dormant→tombstone timing remains lifecycle housekeeping only; wall time does not itself trigger another model attempt. Reconsideration is evidence/event driven.
