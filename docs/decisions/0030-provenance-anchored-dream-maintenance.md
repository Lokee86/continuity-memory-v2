# ADR 0030: Provenance-anchored Dream maintenance

Status: Accepted and implemented, 2026-09-05

Owners: Dream scheduling, ReliquaryRuntimeHost, Memory lifecycle maintenance

Amends ADR 0024's extracted-only Dream backlog model.

## Context

Dream initially integrates a new Memory by comparing it with a bounded historical candidate set. Once that Memory leaves `extracted`, the previous runtime never considers it as a Dream source again. A relation missed during that first bounded comparison can therefore remain undiscoverable even after later graph growth makes the historical pair relevant.

Making every new Memory progressively scan the historical graph would defeat bounded candidate discovery. Dream instead needs low-rate reprocessing of existing Memories without turning imports or old graphs into periodic backlog avalanches.

## Decision

Dream maintenance uses fixed provenance-anchored cooldown epochs.

The current default cooldown is 30 days:

```text
DEFAULT_DREAM_REPROCESS_COOLDOWN_NS = 30 days
```

For a Memory with source/provenance timestamp `P` at wall time `T`:

```text
epoch = floor(max(0, T - P) / cooldown)
```

The epoch is an eligibility window, not a count of work owed.

A newly extracted Memory remains immediately eligible for its initial Dream pass. When that pass succeeds, Dream records the Memory's **current** provenance-relative epoch as satisfied. If a 45-day-old Memory is first processed under a 30-day cooldown, Dream records epoch `1`; it does not enqueue a missed epoch `0` pass. It becomes eligible again when its age crosses 60 days and enters epoch `2`.

For an already processed, active Memory, Dream is eligible only when:

```text
current provenance epoch > last successfully processed Dream epoch
```

If several boundaries were crossed while Dream was inactive, only the current epoch is processed. There is no accumulated maintenance backlog.

Successful Dream processing records the satisfied epoch after relation publication and lifecycle reconciliation. Failed classification or verification does not advance the marker and remains subject to the existing short retry cooldown.

## Persistence

Cooldown state is owner-local append-only metadata keyed by `MemoryId` and satisfied epoch. It is not stored as a Memory revision and therefore does not advance semantic Memory versions or invalidate derived Memory retrieval/community state merely because maintenance ran.

REL and PHY persist the same cooldown record type inside their own containers. On reopen, the latest/highest epoch per Memory is reconstructed.

Divergent REL reconciliation merges cooldown state monotonically by taking the maximum satisfied epoch for each Memory. Cooldown-only divergence does not become a semantic repository-correlation mutation.

## Provenance rules

REL maintenance uses the same authoritative source chronology as Dream candidate processing: persisted `source_time_ns` first, then validated Archive/Episode provenance fallback for legacy REL Memories.

PHY maintenance uses persisted `source_time_ns` only.

`created_at_ns` is not substituted for semantic provenance.

An extracted Memory with no recoverable source timestamp can still receive its initial Dream pass. Once it leaves `extracted`, periodic maintenance remains disabled until authoritative source chronology exists.

## Legacy bootstrap and imports

Old files have no persisted Dream cooldown marker. For a non-extracted legacy Memory, the provenance timestamp still anchors the epoch cycle, while the Memory's existing `updated_at_ns` is used only to infer which provenance-relative epoch was already satisfied before cooldown markers existed. This avoids making a previously processed graph immediately look wholly unprocessed when the feature is introduced.

Imported historical Memories remain `extracted` until their initial Dream pass. That pass satisfies whatever provenance-relative epoch they currently occupy, so old imports do not accumulate one maintenance pass for every historical interval they crossed before import.

Because epoch boundaries are anchored to each Memory's provenance timestamp rather than import time or last-Dream time, historical source timestamps naturally distribute future re-eligibility across the cooldown window.

## Consequences

- Dream remains bounded per pass; candidate limits are unchanged.
- Old Memories periodically get another chance to discover relations to newer graph state.
- A newly processed Memory may become eligible again anywhere from almost immediately to one full cooldown later, depending on its provenance phase.
- Bulk import does not create a historical maintenance debt.
- Dream still owns no durable work queue; only the last satisfied provenance epoch is durable.
- Runtime background status includes both initial extracted work and cooldown-eligible maintenance work.
