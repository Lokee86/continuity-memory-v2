# ADR 0018: Cloud-backed CVA reconciliation

## Status

Accepted — 2026-08-24. Comparison, fresh semantic repacking, derived-state cleanup, Archive/Memory/Insomnia replay, file-to-Memory reconciliation, structured semantic conflict reporting, and safe path-based canonical promotion are implemented; provider discovery/integration remains in progress.

## Purpose

Allow a Warlock workspace CVA to live in user-controlled cloud storage without requiring a Warlock-owned synchronization service.

## Context

Construction makes desktop/phone/tablet access a first-class requirement. Common cloud storage systems already own file transport, offline caching, version history, and conflicted-copy preservation. Reimplementing those responsibilities inside Warlock would add substantial infrastructure and weaken the local/user-owned storage model.

A cloud provider cannot, however, understand the internal semantics of an arbitrary `.cva`. If two devices independently modify copies of the same workspace, the provider may preserve both files but cannot safely merge their logical records.

Continuity can perform that last step because it owns the CVA format and semantic stores.

## Decision

### Cloud storage owns transport

Warlock and Continuity do not implement a general cloud synchronization service for the first multi-device architecture.

The user's selected provider remains responsible for:

- storage;
- upload/download;
- offline caching;
- availability;
- server-side file version history; and
- detection/preservation of conflicting physical copies.

Continuity owns only CVA-specific comparison and reconciliation.

### Workspace identity is the reconciliation boundary

Two CVAs are eligible for reconciliation only when both contain workspace metadata and their stable `WorkspaceMetadata.id` values are equal.

A filename, directory, cloud-provider identity, or display name is not sufficient proof that two files represent the same workspace.

### Physical history is used to detect ordinary cloud divergence

Cloud-conflicted copies normally begin as byte-identical copies of one CVA and then acquire different append-only tails.

The first comparison layer therefore identifies the longest common physical chunk prefix and classifies the pair as:

```text
Identical
LeftExtendsRight
RightExtendsLeft
Diverged
```

This layer is detection only. It does not assume that physical chunk positions or global version numbers remain reusable after divergence.

### Divergent tails must be reconciled semantically

Raw tails must not be concatenated. Each diverged copy may independently allocate overlapping CVA-global, Archive, Memory, or other local version numbers.

The reconciliation engine must decode authoritative records from both tails and replay accepted records into a newly written CVA so the destination allocates valid clocks and physical references.

Conceptually:

```text
base.cva
├── common history
├── left semantic tail
└── right semantic tail
        ↓
semantic classification
        ↓
replay into new CVA
        ↓
validate + sync
        ↓
replace/promote canonical copy
```

### Stable semantic identity drives automatic merge

Reconciliation should prefer existing domain identities and invariants rather than inventing one generic merge rule.

Examples:

- identical stable Node ID + identical record: duplicate, keep once;
- identical stable Node ID + incompatible record: conflict;
- Memory mutation replay with identical record: idempotent duplicate;
- same Memory revision changed incompatibly: conflict;
- unrelated records: preserve both when their domain invariants allow replay;
- content-addressed immutable objects: deduplicate by identity/content.

Each purpose-built store remains responsible for deciding whether an imported/replayed record is valid.

### Derived state should be rebuilt or retired when cheaper and safer

Indexes, vector bindings, caches, and similar derived structures do not need byte-for-byte reconciliation when they can be regenerated from authoritative merged state.

Current divergent reconciliation creates a fresh CVA. Immutable Fragment ranges are revalidated and replayed because they remain valid references to merged Archive nodes. The lexical index is process-local/disposable and therefore rebuilds lazily from those merged Fragments and Files after reopen. Compatibility profiles are immutable vector-space contracts and are preserved. Packed vector matrices, Memory-Vector bindings, Archive-Vector bindings, and Vector Generations are deliberately omitted from the fresh repack because they describe a pre-merge population cut; the result reports `vector_rebuild_required` when either source contained such state. Re-embedding remains an explicit follow-up because reconciliation itself has no embedding endpoint and must not fabricate vector data.

### Reconciliation writes a new file

A divergent merge must not mutate the only canonical copy in place.

The intended lifecycle is:

```text
left.cva + right.cva
        ↓
workspace.merge.tmp.cva
        ↓
Cva::open / full validation
        ↓
sync
        ↓
promote/replace canonical file
```

Original conflicted copies are retained until validation succeeds. `Cva::reconcile_and_promote` now implements this filesystem-path lifecycle: it fingerprints the canonical CVA, builds a hidden sibling candidate through `Cva::reconcile`, syncs/reopens it, verifies the canonical fingerprint is unchanged, creates and syncs a recovery copy, atomically replaces the canonical path, then reopens the promoted CVA before deleting the recovery copy. If any post-replacement finalization step fails, the recovery copy is restored and revalidated before the error is returned. The conflicted source is never modified. This guard detects ordinary changes during preparation but is not a substitute for a general interprocess writer lock or provider-side compare-and-swap.

## Initial implementation

The first code slices add `Cva::compare(left, right)` plus `Cva::reconcile(left, right, output)`.

They currently:

1. open and validate both CVAs;
2. require workspace metadata on both sides;
3. reject mismatched workspace IDs;
4. compare physical chunk histories and identify the common prefix;
5. copy the complete side directly for identical/strict-extension cases;
6. for true divergence, create a fresh workspace CVA rather than copying either physical branch;
7. decode and replay the complete left semantic history plus the right divergent semantic tail, allocating fresh destination clocks;
8. replay source Nodes, attachment-bearing ingested turns, standalone Files, Branch revisions, immutable Episodes, and immutable Fragments through ordinary Archive APIs;
9. decode standalone and grouped Insomnia-produced Memory revisions, then replay them through `publish_memory` so stable IDs/mutation IDs are preserved while destination clocks are newly allocated;
10. re-emit durable Insomnia completion-only receipts after their Memory IDs exist, deduplicating identical receipts and rejecting incompatible completions for the same Episode;
11. replay file-to-Memory links after both endpoint owners exist and preserve compatibility profiles from both copies;
12. omit packed vectors, Memory/Archive vector bindings, and Vector Generations from the repack, while reporting whether a vector rebuild is required;
13. rely on the disposable lexical index to rebuild lazily from replayed merged Fragments/Files;
14. sync and reopen the new output for validation, removing it if reconciliation fails;
15. normalize known semantic replay collisions into public `CvaReconcileConflict` variants carrying stable owner identity and competing state, while leaving corruption/I/O/format failures as ordinary errors;
16. when `reconcile_and_promote` is used, fingerprint the canonical input before/after candidate preparation, write a synced recovery copy, atomically replace the canonical path, reopen/sync the promoted CVA, restore and revalidate the recovery copy if any post-replacement finalization step fails, and clean hidden sibling artifacts after ordinary success/failure.

This is now a functional semantic merge, derived-state cleanup, and safe path-based promotion path for current Archive/Memory/Insomnia/vector ownership, but not yet a complete product-level cloud-conflict workflow.

## Next implementation slices

1. Add provider-facing conflicted-copy discovery around the library comparison/reconciliation/promotion operations.
2. Add a host/runtime rebuild hook that can consume `vector_rebuild_required` and rebuild vectors when a verified embedding endpoint is available.
3. Add Warlock-side presentation/resolution flows for the structured conflicts; Continuity remains responsible only for typed conflict semantics.
4. Test realistic multi-device fixtures, including offline source capture, independent Memory production, attachments, derived-state rebuild, repeated conflict/reconciliation cycles, and provider-mediated file replacement.

## Non-goals

This decision does not introduce:

- a Warlock cloud account requirement;
- a Warlock-hosted canonical database;
- multi-master network replication;
- CRDT semantics for every Continuity store;
- provider-specific synchronization protocols; or
- automatic semantic conflict resolution where two edits are genuinely incompatible.

## Consequences

- Multi-device use can rely on OneDrive, Google Drive, iCloud Drive, Dropbox, SharePoint-backed files, or equivalent storage transports.
- The CVA remains user-owned and portable.
- Continuity gains one bounded responsibility: understanding divergent copies of its own format.
- Automatic reconciliation can improve incrementally without coupling the storage format to any cloud vendor.
- True simultaneous semantic conflicts remain explicit rather than being silently overwritten.

## Verification

Tests now prove that comparison recognizes identical copies, strict extensions, independent divergent tails, and different workspace IDs. Reconciliation tests additionally prove unrelated source Nodes and attachments merge, immutable Episodes preserve Memory provenance, standalone and grouped Insomnia-produced Memory revisions are re-ticketed correctly, file-to-Memory links replay after their targets, identical completion receipts deduplicate, incompatible Branch/Memory/completion revisions fail closed, and failed merge output is removed. Derived-state coverage proves a divergent result is a fresh repack, preserves/revalidates Fragments and compatibility profiles from both sides, rebuilds lexical retrieval from merged Fragments, removes packed/Memory/Archive vector state and Vector Generations, and reports that vector rebuilding is required.

Remaining work is provider-level conflicted-copy discovery/fixtures, host-triggered vector rebuilding, Warlock-side conflict presentation/resolution, repeated reconciliation-cycle testing, and eventual stronger writer coordination if real multi-process/cloud races require it.

## Related docs

- [ADR 0003](0003-layered-version-clocks-and-local-ancestry.md)
- [ADR 0005](0005-cva-composition-and-packed-vector-objects.md)
- [ADR 0017](0017-cva-workspace-and-warlock-host-application.md)
- [Architecture](../architecture.md)
- [Roadmap](../roadmap.md)
