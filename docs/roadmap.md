# Roadmap

Parent index: [Documentation index](INDEX.md)

## Purpose

This document owns future implementation work for Reliquary. Completed behavior does not belong here; current behavior is documented in [Architecture](architecture.md), [Rust API](api.md), [Storage format](storage-format.md), [Repo-local CLI](cli.md), and [Current limitations](current-limitations.md).

## Overview

Reliquary remains the purpose-built semantic/storage subsystem inside Warlock. Project-file history belongs to Lore or Git; REL/PHY retain independent semantic state and history. Future work should deepen that separation rather than rebuilding project VCS inside Reliquary.

The immediate priorities are to finish the remaining project-repository boundary cleanup, narrow REL/PHY synchronization to semantic-database concerns, complete the application/runtime surfaces Warlock still needs, and then build higher-level context composition such as Ego on top of stable owner-local retrieval.

## Immediate priorities

### 1. Retire duplicate project-file VCS responsibilities

Remove or simplify REL machinery whose remaining purpose duplicates authoritative Lore/Git project history.

Candidates include:

- project/blob history retained only for filesystem recovery;
- project-file ancestry, branch, merge, or replay behavior superseded by repository history;
- file-state reconstruction that can resolve through an exact `ProjectFileRef`; and
- whole-project recovery behavior already provided by the project repository.

Do **not** collapse the REL semantic timeline into project-repository history. Preserve Memory revisions, Graph versions, Episode/transcript history, provenance, semantic supersession, vector generations, crash recovery, and other semantic/database-local history.

Project creation, repository discovery, Lore/Git mutation policy, upload materialization, and repository checkout behavior are Warlock responsibilities governed by [ADR 0027](decisions/0027-warlock-project-repositories-and-reliquary-storage-boundary.md) and [ADR 0028](decisions/0028-project-folder-and-repository-bootstrap-contract.md), not new Reliquary semantic owners.

### 2. Re-scope REL/PHY synchronization

Reassess whole-file reconciliation around the smaller remaining problem of divergent semantic databases now that project-file history is external.

Future work should:

- exclude project-file history already authoritative in Lore/Git;
- retain owner-explicit validation for Archive, Memory, Graph, Episode, provenance, and other semantic owners;
- preserve safe promotion/recovery behavior until a simpler replacement is proven;
- treat REL/PHY transport independently from project-repository transport;
- define cross-device conflict behavior without assuming project-repository ancestry resolves semantic conflicts; and
- determine whether cloud-drive conflicted-copy transport remains sufficient or a coordination layer is justified.

The current reconciliation API remains compatibility behavior; do not expand it into a second project VCS.

### 3. Complete the live runtime seam

Finish the host-facing runtime behavior that is not yet covered by the in-process `InteractionRuntime` / `ReliquaryRuntimeHost` boundary:

- adapter reconnect/resume and adapter migration around durable session/message cursors;
- normalized tool, session, and generated-artifact events without transport-specific semantic authority;
- explicit cancellation and shutdown behavior for long-running inference work;
- capability caching where safe;
- bounded memory-control operations needed by hosts; and
- runtime status/diagnostics sufficient for Warlock to expose failures and recovery state.

Do not turn Reliquary into an independently deployed service merely to host these capabilities. Any future IPC/server boundary requires a separate architectural decision.

### 4. Complete the workspace management API

Expose the remaining concrete owner operations Warlock needs without creating a generalized mutable semantic object layer.

Needed surfaces include:

- broader conversation/session management where current transcript primitives are insufficient;
- file inventory, import/export, source provenance, and project-file reference inspection;
- Memory inventory, provenance, lifecycle inspection, and permitted manual lifecycle actions;
- health, verification, statistics, and diagnostics;
- selective import/export of user-owned semantic state; and
- safe unlink/removal only after retention/history semantics are defined.

The management layer composes existing owners; it does not become a new semantic owner.

### 5. Production import and interoperability adapters

Add normalized ingestion for historical and external interaction sources:

- production ChatGPT import;
- Claude/provider export import;
- Codex/Hermes and other structured agent-session imports where available;
- ACP interoperability over the same normalized live interaction seam;
- deterministic re-import/idempotency rules;
- explicit handling of edits, replacements, deletions, and provider-specific branches; and
- artifact provenance that distinguishes the initiating interaction, produced artifact, and artifact evidence.

ACP remains an adapter, not a canonical storage schema. See [ADR 0015](decisions/0015-acp-inline-interaction-stream.md) and [ADR 0016](decisions/0016-native-product-surface-and-shared-interaction-runtime.md).

### 6. File usability

Add product-facing file operations without reintroducing REL-owned project-file history:

- standalone add/import and export through management surfaces;
- user-visible path/folder/tree organization;
- rename/move behavior that preserves historical reference meaning;
- extraction pipelines for supported document/file types;
- file-content indexing and retrieval separated from filename metadata; and
- generated-artifact provenance and lifecycle policy.

### 7. Runtime and security hardening

Complete production hardening:

- OAuth token refresh;
- operating-system credential-store backed master-key persistence;
- default OS application/config locations;
- provider retry/backoff and rate-limit adaptation;
- explicit provider/model/credential/reasoning fallback routes per capability;
- runtime observability without leaking secrets or user content;
- crash-safe restart/recovery for host-owned background work; and
- authentication/authorization only if a future local IPC/API boundary is introduced.

## Scope and ownership evolution

Reliquary already has typed durable scope identity; future work is about how those owners are composed and governed.

Near-term scope work should:

- support hierarchy among multiple active RELs so project context does not cross-contaminate while Organization scopes can own Projects;
- keep durable ownership separate from retrieval visibility and mutation authority;
- define scope creation/editing and hierarchy-management surfaces;
- extend learned-state routing beyond the current conservative User/Project boundary only when the destination authority rules are explicit;
- keep governed Organization state distinct from learned observations; and
- define explicit cross-file Memory/source export and lineage semantics.

Connection/relationship scope expansion is not an immediate target. Existing typed Connection identity remains a supported file kind, but new relationship-routing/hierarchy behavior stays mothballed under [ADR 0029](decisions/0029-active-rel-hierarchy-and-deferred-connection-scope.md) until a concrete product requirement reactivates it.

## Reliquary / Phylactery product transition

Remaining product/migration work:

1. update all Warlock file creation/open/association UX to use authoritative typed REL/PHY identity before retrieval/context assembly;
2. warn when filename hints disagree with authoritative internal type/scope;
3. define cross-file Memory/source export and lineage semantics between REL and PHY; and
4. decide whether the internal/back-compat `Cva` terminology should eventually be removed from public-facing APIs and documentation.

Do not implement `.phy` as a Project Reliquary with provenance fields merely made nullable. Shared mechanics and separate semantic validation remain required.

## Insomnia follow-up

Routine prompt tuning on the adversarial fixture remains frozen. Future work is validation or a new measured capability boundary:

- run the full 66-Episode gold-v3 corpus as milestone confirmation;
- recalibrate worker concurrency and model/reasoning cost for the selected production route mix; and
- add targeted verifier/repair, selective voting, deterministic clause preprocessing, ambiguity routing, supersession resolution, provenance verification, or stronger/fine-tuned selection only when production failures justify the added inference.

Do not resume benchmark-specific prompt squeezing merely to chase stochastic fixture misses.

## Echo retrieval surface

Expose bounded source-scoped Echo expansion to higher-level context assembly.

Requirements:

- upstream source/provenance selection must determine what Echo is eligible;
- Echo must remain cold by default and non-authoritative;
- no global reasoning-trace search index should be introduced by default; and
- provider continuation state must remain separate from historical reasoning evidence.

See [ADR 0014](decisions/0014-echo-historical-reasoning-traces.md).

## Dream follow-up

Treat further Dream architecture as measurement-driven rather than continuing unconditional inference expansion.

Potential future work includes:

- bounded reconsideration only for observed unresolved/ambiguous cases;
- model-enriched temporal interpretation only where deterministic chronology is insufficient;
- derived temporal acceleration only if measured candidate cost warrants it;
- additional relationship provenance/evidence persistence where product inspection requires it; and
- new relation classes only with explicit publication and lifecycle semantics.

The shipped design and validation history belong to current architecture/reference and retained validation material, not this roadmap.

## Memory-web and retrieval integration

Future integration work:

- attach reusable `MemoryRetrievalIndex` lifecycle to Ego/runtime so repeated queries reuse derived indexes and rebuild deterministically when stale;
- compose owner-local REL and PHY retrieval above those primitives without creating cross-owner Dream/Graph authority;
- measure release-mode latency against `GlobalExact` during rollout;
- consider explicit lower-cost routing modes only if production economics justify them;
- tune the implemented Community-lineage continuation/material-change thresholds only from real archive behavior; continuity-preserving Community IDs remain unnecessary while derived lineage is sufficient;
- add incremental Community maintenance only if measured scan-and-merge cost becomes material; and
- preserve explicit user Community names as metadata only, never Memory-Web authority; current lineage inheritance must remain conservative around ambiguous splits/merges.

## Ego

Build active context synthesis after owner-local retrieval and scope hierarchy are stable enough to provide trustworthy inputs.

Ego should:

- synthesize each active Memory Web into its own bounded context block;
- compose multiple active RELs according to explicit hierarchy rather than indiscriminate union;
- provide bounded cross-session/recent-session context;
- include user-global Phylactery context through an explicit owner lane;
- keep source/provenance access available without flooding the default prompt; and
- support optional personality synthesis without making personality a semantic authority over project/user facts.

Detailed Ego architecture belongs in its own design owner once implementation boundaries are selected.

## Storage, scale, and historical recovery

Keep physical optimization measurement-driven behind existing semantic boundaries:

- mapped/segmented vector scanning and ANN acceleration;
- persistent lexical acceleration only if reopen/query measurements justify it;
- quantized searchable representations;
- explicit vector-generation retirement;
- REL/PHY packing, compaction, retention, and recovery; and
- derived-state rebuild/invalidation policy.

Whole-REL historical restore/branching must be defined in semantic-database terms rather than reintroducing project-file VCS. See [Versioning, historical cuts, and rollback](version-history-plan.md).

## Product acceptance gates

### Usable local product

A non-developer can create/open a Warlock project, converse through Warlock, work with files, browse conversations and durable knowledge, and understand basic provenance without CLI use.

### Interoperable product

Equivalent normalized interaction history can enter through Warlock-native execution and at least one external adapter without protocol-specific semantic records.

### Durable live product

Long-running capture, background processing, reconnect, crash/restart, and provider failures have explicit tested behavior and do not silently lose acknowledged source events.

### Expandable semantic product

New semantic owners remain purpose-built, use stable cross-owner IDs, and do not require a generalized REL root/dependency framework.

## Open decisions

- Exact hierarchy representation and context-resolution policy for multiple active RELs and Organization-owned Projects.
- Cross-file Memory/source lineage and export permissions among REL and PHY owners.
- Whether `Cva` remains only an internal/back-compat term.
- How learned-state ownership expands beyond User/Project without conflating ownership with governance or authorization.
- Normalized interaction vocabulary for tool/session/artifact events beyond completed turns.
- REL/PHY synchronization and conflict transport across devices.
- Which semantic mutations are safe to expose as direct user actions.
- Adapter failure policy and privacy controls for automatic capture.
- Artifact provenance vocabulary across uploaded, generated, imported, and provider-managed artifacts.
- Archive checkpoint representation, packing/compression choices, and retention policy.
- Whole-REL restore/timeline terminology and retention semantics.

## Explicitly not planned

The current roadmap does not include:

- embedding Lore as the general physical substrate for REL/PHY;
- migrating Memory/Graph/vector/Echo storage onto Lore;
- maintaining a hidden Lore repository when Git is the selected project-history authority;
- a second permanent uploaded-file blob store;
- a virtual/copy-on-write filesystem or filesystem driver inside Reliquary; or
- a mandatory independently deployed Reliquary service.

## Related docs

- [Architecture](architecture.md)
- [Current limitations](current-limitations.md)
- [Versioning and rollback plan](version-history-plan.md)
- [ADR 0014](decisions/0014-echo-historical-reasoning-traces.md)
- [ADR 0015](decisions/0015-acp-inline-interaction-stream.md)
- [ADR 0016](decisions/0016-native-product-surface-and-shared-interaction-runtime.md)
- [ADR 0017](decisions/0017-cva-workspace-and-warlock-host-application.md)
- [ADR 0019](decisions/0019-cloud-backed-cva-reconciliation.md)
- [ADR 0027](decisions/0027-warlock-project-repositories-and-reliquary-storage-boundary.md)
- [ADR 0028](decisions/0028-project-folder-and-repository-bootstrap-contract.md)
- [ADR 0029](decisions/0029-active-rel-hierarchy-and-deferred-connection-scope.md)

## Notes

This file is future-only by policy. When a roadmap item ships, remove it from this document and document the resulting behavior in the current-state owners instead.
