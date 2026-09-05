# ADR 0030: Provenance-anchored Dream maintenance

Status: Accepted and implemented, 2026-09-05

Owners: Dream scheduling, Dream candidate discovery, ReliquaryRuntimeHost, Memory lifecycle maintenance

Amends ADR 0024's extracted-only Dream backlog model.

## Context

Dream initially integrates a new Memory by comparing it with a bounded historical candidate set. Once that Memory leaves `extracted`, an extracted-only runtime never considers it as a Dream source again. A relation missed during that first bounded comparison can therefore remain undiscoverable even after later graph growth introduces a better comparison opportunity from another Memory.

Making every new Memory progressively scan the historical graph would defeat bounded candidate discovery. Re-running the same top candidates would also be ineffective: already-related pairs, and pairs previously classified as `none`, would repeatedly consume the same bounded slots.

Dream therefore needs low-rate reprocessing of existing Memories while ensuring each maintenance pass explores unresolved pairs rather than replaying settled ones.

## Decision

Dream maintenance uses fixed provenance-anchored cooldown epochs. The current default cooldown is 30 days:

```text
DEFAULT_DREAM_REPROCESS_COOLDOWN_NS = 30 days
```

For a Memory with authoritative source/provenance timestamp `P` at wall time `T`:

```text
epoch = floor(max(0, T - P) / cooldown)
```

The epoch is an eligibility window, not a count of work owed.

A newly extracted Memory remains immediately eligible for its initial Dream pass. When that pass succeeds, Dream records the Memory's **current** provenance-relative epoch as satisfied. If a 45-day-old Memory is first processed under a 30-day cooldown, Dream records epoch `1`; it does not enqueue a missed epoch `0` pass. It becomes eligible again when its age crosses 60 days and enters epoch `2`.

For an already processed active Memory with authoritative source chronology, Dream is eligible only when:

```text
current provenance epoch > last successfully processed Dream epoch
```

If several boundaries were crossed while Dream was inactive, only the current epoch is processed. There is no accumulated maintenance backlog.

## Chronology and fail-safe cadence

REL resolves semantic source chronology in this order:

```text
recoverable Archive/Episode provenance
    -> persisted source_time_ns
```

PHY has no Archive/Episode owner, so its semantic chronology is persisted `source_time_ns` when available.

Dream also records the wall-clock timestamp of every successful Dream pass. If neither REL provenance nor persisted `source_time_ns` is available, that **last successful Dream processing time** becomes the maintenance cadence anchor so the Memory can still become eligible again after one cooldown interval.

The Dream processing timestamp is scheduling metadata only. It is never substituted for semantic chronology in temporal analysis, duplicate ordering, causality, supersession, or any other reasoning that requires source time.

For legacy files that predate persisted Dream-processing timestamps, `updated_at_ns` is used only as a one-time non-flooding maintenance bootstrap when no better source or Dream timestamp exists. After the next successful Dream pass, the real Dream timestamp becomes the fallback cadence anchor.

## Settled-pair suppression

Dream persists an owner-local unordered pair ledger for successfully evaluated Memory pairs. A pair enters the ledger after classifier/verifier evaluation has completed and publication has succeeded, including classifications that produce no Graph edge such as `none` or a withheld relation.

Candidate discovery excludes, **before lane ranking and final-slot allocation**:

- any candidate already connected to the source by an active Graph edge in either direction; and
- any candidate whose unordered pair with the source is already in the Dream evaluation ledger.

This prevents maintenance from repeatedly spending the 12-candidate default budget on already-settled pairs. Because Memory semantic bodies are immutable for one `MemoryId`, an edited/corrected Memory receives a new ID and naturally becomes eligible for fresh pair evaluation.

## Persistence

Cooldown state is owner-local append-only metadata keyed by `MemoryId`. The current record stores both the last satisfied provenance epoch and the last successful Dream processing timestamp. It is not a Memory revision and therefore does not advance semantic Memory versions or invalidate derived Memory retrieval/community state merely because maintenance ran.

The pair ledger is also owner-local append-only metadata. Its identity is the canonical unordered pair of two stable `MemoryId` values. It consumes no Memory, Graph, Archive, or global semantic version.

REL and PHY persist these maintenance records inside their own containers. On reopen, cooldown records merge monotonically and pair records rebuild the evaluated-pair set.

Divergent REL reconciliation merges cooldown state by taking the maximum satisfied epoch and latest available successful-processing timestamp per Memory. Evaluated-pair histories are unioned. Maintenance-only divergence does not become a semantic repository-correlation mutation and does not require vector rebuilding.

## Imports and legacy state

Imported historical Memories remain `extracted` until their initial Dream pass. That pass satisfies whatever provenance-relative epoch they currently occupy, so old imports do not accumulate one maintenance pass for every historical interval crossed before import.

Because epoch boundaries are anchored to each Memory's provenance timestamp rather than import time or last-Dream time, historical source timestamps naturally distribute future provenance-backed re-eligibility across the cooldown window.

For provenance-less Memories, successful Dream processing time provides the explicit recurring fallback instead of disabling maintenance permanently.

## Consequences

- Dream remains bounded per pass; candidate limits are unchanged.
- Old Memories periodically get another chance to discover relationships to newer or previously unexamined graph state.
- A provenance-backed Memory may become eligible again anywhere from almost immediately to one full cooldown later, depending on its provenance phase.
- A provenance-less Memory remains maintainable through the last-successful-Dream fallback.
- Bulk import creates no historical maintenance debt.
- Existing Graph neighbours and previously evaluated no-edge pairs do not consume future Dream candidate slots.
- Dream still owns no durable work queue; only compact owner-local maintenance state is durable.
- Runtime background status includes both initial extracted work and cooldown-eligible maintenance work.
