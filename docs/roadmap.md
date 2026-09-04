# Roadmap

Parent index: [Documentation index](INDEX.md)

## Purpose

This document owns future implementation work for Reliquary. Completed behavior does not belong here; current behavior is documented in [Architecture](architecture.md), [Rust API](api.md), [Repo-local CLI](cli.md), and [Current limitations](current-limitations.md).

## Overview

The immediate storage/history direction is split cleanly between project repositories and Reliquary/Phylactery semantic state. Every Warlock project gets project history from Git or Lore; REL/PHY keep their purpose-built semantic storage rather than migrating wholesale onto a VCS substrate.

Further investment in REL machinery that duplicates project/file VCS is frozen except for compatibility and correctness work. REL/PHY semantic history, historical cuts/versioning, recovery, and domain-local revision machinery remain first-class requirements. Productization and semantic-quality work may continue against the existing REL/PHY semantic APIs and `ObjectRef` physical seam.

Lore is the automatic managed project repository unless the advanced `Use Git for repository` option is selected. Git remains the explicit/user-owned repository case. Reliquary continues to own Archive, Memory, provenance, Insomnia, Dream, Graph, Echo, retrieval, scopes, transcript, and other agent/context state. See [ADR 0027](decisions/0027-warlock-project-repositories-and-reliquary-storage-boundary.md).

## Product direction

Reliquary is moving from a storage/retrieval substrate toward the durable workspace/context layer of the broader Warlock product.

One Reliquary REL maps to one non-user Warlock scope/workspace; the current Warlock host uses Project RELs. Warlock owns the native application surface and long-lived application orchestration; Reliquary remains the Rust semantic/storage subsystem linked into that application core. External agent protocols such as ACP remain interoperability adapters into the same normalized interaction seam and are not canonical storage schemas. See [ADR 0016](decisions/0016-native-product-surface-and-shared-interaction-runtime.md) and [ADR 0017](decisions/0017-cva-workspace-and-warlock-host-application.md).

## Immediate roadmap — universal project repository boundary

ADR 0027 replaces the planned wholesale REL/PHY-to-Lore migration. The completed Lore feasibility spike remains useful evidence and implementation material, but Lore now belongs at the **project repository** boundary rather than beneath every Reliquary semantic owner.

### Target operating model

```text
Warlock project
|
+-- project working tree
|    |
|    +-- Git repository       (existing/explicit advanced case)
|    |
|    `-- Lore repository      (automatic default otherwise)
|
+-- project REL
|    `-- transcript, Memory, provenance, Graph, Echo, vectors,
|        semantic history/versioning, runtime state
|
`-- user PHY
     `-- user-global Memory and associated semantic state
```

Every project therefore has one authoritative project-file history system plus an independent REL semantic history. Reliquary stores references to historical repository file states when transcript/provenance needs exact project context, but it does not duplicate project-file history internally.

The two timelines are correlated rather than merged. A REL historical cut/checkpoint may carry the associated Lore/Git revision so historical reconstruction can answer both what Warlock knew and what project state that knowledge was grounded against.

### 1. Preserve purpose-built REL/PHY storage

Do **not** migrate Memory bodies, Graph records, Episodes/Fragments, Echo, vectors, compatibility profiles, transcript, or other semantic owners onto Lore merely to share a physical/VCS substrate.

Requirements:

- keep the existing purpose-built REL/PHY semantic and physical storage architecture;
- retain the opaque `ObjectRef` seam so semantic owners remain decoupled from physical offsets;
- continue measurement-driven physical-store improvements behind that seam when justified;
- preserve domain-local revision/version semantics where they carry application meaning; and
- avoid new abstractions that make project-repository history authoritative over Memory/Graph/etc.

The completed `ObjectRef` seam remains valid architecture independent of Lore.

### 2. Preserve the Lore feasibility work as project-repository evidence

The isolated Lore spike has already established that the selected Lore revision can support the repository capabilities Warlock needs, including ordinary working-tree reconstruction, branches/history/merge, chunking/dedup/compression, binary revisions, copy/reopen, interrupted-tail recovery, and divergent-history merge from a known common ancestor.

The spike and cloud-transport benchmark should be retained as implementation evidence, not expanded into production REL/PHY substrate migration.

Future Lore work should move toward the managed project-repository integration rather than migrating semantic Reliquary backing objects.

### 3. Implement the Project create/open bootstrap contract

ADR 0028 defines the concrete Project REL location and repository bootstrap behavior.

A newly created Project REL must be associated with an ordinary project folder and stored at:

```text
<project-folder>/.warlock/<project-name>.prj.rel
```

`.warlock/` is Warlock semantic/runtime state and must be excluded from project repository tracking before any automatic project commit.

Default creation path:

1. user selects/creates a project folder;
2. resolve existing Lore ownership or initialize Lore for that folder;
3. configure `.warlock/` exclusion;
4. create the `.warlock/` directory and Project REL;
5. create the initial Lore project commit/checkpoint; and
6. record the resulting repository identity/revision in REL association/history metadata so the project and semantic timelines can be correlated.

Advanced creation options are exactly:

- `Use Git for repository`
- `Manually manage Lore repository`

They are mutually exclusive.

`Use Git for repository` must detect the Git repository owning/containing the selected project folder, fail clearly if no usable Git repository is found, create the REL under the selected folder's `.warlock/`, exclude that directory using repository-local Git exclusion where practical, record repository association/current revision, and make **no automatic Git commit**.

`Manually manage Lore repository` still performs Lore detection/initialization, `.warlock/` exclusion, REL creation, and the initial Lore commit so the project starts from a defined state. Warlock now persists that repository as externally managed in the Project correlation. Routine automatic Lore checkpoints are disabled: exact content already present in the current Lore revision can be referenced, while new attachment bytes must be added/checkpointed through explicit Lore management before Warlock will attach them.

Existing repository discovery must avoid nested Lore repositories. The selected project folder may be below the actual repository root; the REL stays under the selected project folder while repository-relative operations use the resolved repository root.

Open/reopen must validate the recorded repository kind/identity against the repository actually present. A missing or mismatched repository must be surfaced rather than silently replaced. Moving the complete project folder remains supported because durable repository identity and relative structure, not one absolute path, define the association.

See [ADR 0028](decisions/0028-project-folder-and-repository-bootstrap-contract.md).

### 4. Repository/REL correlation seam — implemented

Reliquary now exposes repository-neutral `ProjectRepositoryRef`, `ProjectRevisionRef`, `ProjectFileRef`, `ProjectRepositoryManagement`, `RelSemanticCut`, and `ProjectRevisionCorrelation` types. A Project REL can append an idempotent correlation between its current semantic cut and one exact project-repository revision plus its repository-management policy without making Lore or Git authoritative over semantic storage.

The current semantic cut records the REL file-global, Archive, and Memory watermarks. The project revision records repository kind, durable repository identity, the selected Warlock project folder as a repository-relative path, and the opaque revision/commit reference.

Correlation records are append-only REL metadata, survive reopen, and do not allocate a new semantic global version merely for recording the cross-history link. Strict-copy/fast-forward reconciliation preserves the existing correlation history verbatim. A true divergent semantic repack cannot reuse old REL cut numbers, so it compares the `(ProjectRevisionRef, ProjectRepositoryManagement)` sequence for prefix compatibility, ignores the obsolete branch-local cuts, and emits one fresh correlation at the final merged REL cut using the latest compatible project revision and management policy; truly divergent repository/policy sequences fail closed.

Warlock's managed-Lore bootstrap resolves the actual Lore repository ID and current revision after the initial commit (or from an already-existing ancestor Lore repository) and records that correlation before Project creation succeeds.

`ProjectFileRef` is now adopted by the transcript/Archive/provenance attachment path. Project-backed attachment registration persists FileId-to-`ProjectFileRef` bindings and attachment metadata in REL while deliberately omitting duplicate payload bytes. Historical FileId identity includes repository kind/ID, project subtree, repository revision, repository-relative path, content hash, byte length, filename, and MIME metadata, so identical bytes at different repository revisions remain distinct historical references. Correlation history also fails closed if repository kind or durable repository ID changes; the selected `project_path` may evolve within the same repository.

### 5. Make uploads normal project files in Lore projects — implemented

Uploaded files are context-ingestion events, not a separate permanent blob store.

For Lore-managed projects:

1. hash the incoming bytes;
2. search eligible tracked files in the current project state for exact-content matches;
3. ignore inappropriate dedupe targets such as untracked/ignored/generated/temp locations and a small set of obviously disposable filename patterns;
4. reuse the first deterministic eligible exact match when one exists;
5. otherwise materialize the upload under `uploads/` using deterministic collision handling;
6. checkpoint the resulting project state; and
7. point transcript/provenance at the exact historical repository file state used for context.

Once materialized, the file is an ordinary project file and may be moved, renamed, edited, or deleted normally. Managed-Lore upload ingestion now implements exact-content reuse, deterministic `uploads/` materialization/collision naming, automatic Lore commit, exact historical reads with content-hash verification, transcript attachment plumbing, and Memory provenance attachment views. Existing-project reads/uploads require the already-associated Lore repository and do not initialize a replacement if `.lore` is missing.

### 6. Preserve exact upload context in Git without silent branch commits — implemented

For Git projects:

- exact-content matches already tracked at the current commit can be referenced directly;
- otherwise materialize the upload under `uploads/` so it is available to the project/agent;
- do not silently commit the file onto the user's active branch;
- capture the exact uploaded bytes through a Warlock-owned Git snapshot/ref or equivalent Git-native historical object/reference mechanism; and
- make any later normal Git add/commit/move operations explicit.

Do not introduce a secondary Lore repository or REL-local uploaded-file blob store for this case.

Warlock now implements this boundary for Git-associated Project RELs. Existing exact tracked content is referenced at the current `HEAD` commit only when the working-tree bytes still match. Otherwise the upload is materialized under `uploads/`, written as a Git blob, overlaid onto `HEAD` through a temporary index, and captured as a complete tree object anchored at `refs/warlock/snapshots/<tree-id>`. The user's active branch, `HEAD`, and real index are not mutated, including when the user already has staged changes. Historical reads resolve the recorded tree/commit plus repository-relative path and verify the stored SHA-256 content identity. Existing-project upload/read paths require the durable Warlock Git repository identity and fail closed when it is missing or mismatched.

### 7. Preserve REL semantic history; retire only duplicate project VCS

As repository-backed equivalents become available, remove or simplify REL machinery whose primary purpose is duplicating project repository capabilities.

Candidates include:

- project/blob file-history ownership inside REL;
- project-file ancestry/history reconstruction;
- project-file branch/merge/replay machinery;
- file-state history that can instead resolve through Lore/Git revisions; and
- whole-project filesystem recovery paths whose remaining purpose is covered by the project repository.

Do **not** remove the REL timeline. Preserve the machinery required for coherent historical semantic cuts, restore/rollback/branch-after-restore, Memory revisions, Graph versions, Episode/transcript history, provenance, semantic supersession, vector generations, runtime durability, crash recovery, database-local checkpoints, and REL/PHY semantic reconciliation.

Add a narrow correlation from REL historical cuts/checkpoints to the relevant Lore/Git project revision where historical project context is required. This is a cross-history reference, not a shared version clock or a project-tree copy inside REL.

### 8. Re-scope REL/PHY reconciliation as semantic database synchronization

`Cva::compare`, `Cva::reconcile`, and `Cva::reconcile_and_promote` remain implemented current-format compatibility behavior, but the path is frozen for broad new project/file functionality.

Once project files universally live behind Git/Lore history, reassess reconciliation around the smaller remaining problem: divergent agent/context databases.

Future work should:

- stop replaying project-file history that is authoritative in the project repository;
- retain owner-explicit Memory/Graph/Episode/etc. semantic validation where required;
- preserve safe database promotion/recovery behavior until a simpler replacement is proven;
- treat REL/PHY cloud transport independently from project-repository transport; and
- avoid assuming Lore project ancestry can resolve semantic conflicts inside REL/PHY.

### Explicitly not planned

The immediate roadmap does not include:

- embedding Lore as the general physical substrate for REL/PHY;
- migrating Memory/Graph/vector/Echo storage onto Lore;
- maintaining a hidden Lore repository after `Use Git for repository` has selected Git as the Warlock project-history authority;
- a separate permanent uploaded-file store;
- a virtual or copy-on-write filesystem;
- filesystem drivers; or
- mandatory hosted/server infrastructure for the project repository.

## Near-term productization sequence

### 1. Shared live interaction runtime

Deliver long-lived live-session execution and orchestration above the normalized interaction boundary.

Required behavior:

- automate adapter reconnect/resume and adapter migration around the explicit durable session/message cursor;
- define normalization/ownership for supported tool events, session events, and generated artifacts without making any transport protocol semantically authoritative;
- wake and finalize inactivity-driven Episode work continuously;
- run Memory/vector work continuously in the background;
- cache verified model/embedding capabilities safely;
- expose the narrow explicit memory-control operations needed by hosts;
- define heartbeat, cancellation, retry, and shutdown behavior for long-running inference work.

### 2. Workspace/REL application API

Expose the concrete Rust operations Warlock needs to manage one REL-backed workspace without introducing a generalized database abstraction or mandatory service layer.

The application surface should compose explicit owner operations for:

- broader conversation/session management beyond the implemented derived conversation summaries and exact-leaf transcript reads;
- file inventory, import, export, source provenance, and later organization;
- Memory inventory, provenance, lifecycle inspection, and permitted manual lifecycle actions;
- health, verification, statistics, and diagnostics;
- selective import/export of user-owned state;
- safe unlink/removal operations only after retention/history semantics are defined.

The management layer must remain an application/service surface over concrete owners, not a new semantic owner or generic mutable object store.

### 3. Warlock host integration

Reliquary is now linked directly into the Warlock v2 Rust application core. One Project REL is the durable workspace file, and Warlock currently implements create/open/close/reopen plus durable conversation list/start/resume/user-turn/reopen through the public Reliquary runtime seam.

Remaining host integration work includes:

- provider/agent execution and durable assistant turns;
- interaction attachments and explicit multi-leaf branch selection;
- file/artifact access;
- Memory/knowledge access;
- provenance/history inspection;
- model/agent configuration seams required by the host;
- background Episode/Insomnia execution;
- basic health/status visibility; and
- maintain existing REL/PHY cloud-conflict handling only as needed for current-format compatibility while semantic-database synchronization is reassessed separately from project repository history.

`Cva::compare`, `Cva::reconcile`, and `Cva::reconcile_and_promote` remain current behavior and are documented by [ADR 0019](decisions/0019-cloud-backed-cva-reconciliation.md), but their architecture is now **frozen rather than extended**. Do not add project-repository history or uploaded-file storage to this path. The future problem is narrower REL/PHY semantic-database reconciliation because Lore/Git now owns project-file history universally; see [ADR 0027](decisions/0027-warlock-project-repositories-and-reliquary-storage-boundary.md).

The TypeScript presentation layer consumes Warlock application commands/state and must not parse REL files or implement Reliquary lifecycle semantics directly.

### 4. ACP interoperability adapter

Implement ACP as a first-class external agent/client adapter over the same live runtime used by the Warlock-native interaction path.

Required work includes:

- map ACP session/message identity onto the normalized interaction/session contract;
- capture supported observable messages, attachments, tool events, and surfaced reasoning without treating ACP metadata as semantic authority;
- map ACP stream/update semantics onto the shared runtime's completed-message assembly and durable acknowledgement boundary;
- support retrieval/context injection without rewriting authoritative source events;
- define reconnect/resume and fail-open/fail-closed behavior;
- isolate Draft/protocol churn inside the adapter;
- preserve privacy/consent controls for automatic capture.

ACP-specific behavior remains constrained by [ADR 0015](decisions/0015-acp-inline-interaction-stream.md), as amended by ADR 0016.

### 5. Production import and adapter layer

Add historical/closed-provider ingestion adapters that feed the same normalized source contract as live interaction paths.

Priorities:

- production ChatGPT import;
- Claude/provider export import;
- Codex/Hermes and other agent-session adapters where structured history is available;
- provider research/artifact provenance, including a distinction between the initiating workflow, produced artifact, and artifact provenance;
- deterministic re-import/idempotency rules;
- explicit handling of edits, replacements, deleted messages, and provider-specific branches.

### 6. File usability

Add product-level file usability:

- standalone file add/import and export through management surfaces;
- user-visible path/folder/tree organization semantics;
- rename/move semantics that preserve underlying immutable content identity;
- extraction pipelines for supported document/file types;
- file-content indexing and retrieval with explicit separation between filename metadata and content-derived indexes;
- generated-artifact provenance and later artifact lifecycle policy.

### 7. Production runtime and security hardening

- OAuth token refresh for provider credentials;
- operating-system credential-store backed master-key persistence;
- default OS application/config locations;
- provider retry/backoff and rate-limit adaptation;
- explicit credential/model fallback policy per capability: ordered fallback routes must bind provider + model + credential + reasoning together, distinguish retryable rate/quota/transport failures from invalid auth or semantic/model incompatibility, preserve the separate Insomnia-main / metadata-ownership / Dream capability contracts, apply cooldown/backpressure rather than terminalizing temporary quota exhaustion, and expose the active/fallback route in runtime status and diagnostics;
- runtime observability without leaking secrets or user content;
- crash-safe service restart and background-work recovery;
- explicit local IPC/API authentication and authorization if a service boundary is exposed.

## Persistent scope implementation boundary

The broader Warlock scope model now treats **scope as durable state ownership**, separate from retrieval, access, and mutation authority.

Current durable owners are:

- **User / Phylactery (`.phy`):** user-global Identity state.
- **Organization / Reliquary (`.org.rel`):** durable organization-wide operational and governed state that is not project-specific.
- **Project / Reliquary (`.prj.rel`):** durable state belonging to one bounded body of work.
- **Connection / Reliquary (`.con.rel`):** durable state belonging to a relationship between parties/scopes.
- **Session/interaction evidence:** durable source/history state, but not automatically an independent scope.

Organization and Connection may contain both learned/synthesized state and explicitly governed state. The implementation must preserve the authority distinction: repeated observed behaviour must never silently become organizational policy, permission, contract terms, standing exceptions, or other authoritative instructions.

Connections use extensible classification plus directional participant roles rather than rigid `VendorConnection`/`ClientConnection` subclasses. Roles such as client or vendor may influence default context-graph behaviour, but hierarchy is represented by typed scope edges rather than by the Connection type itself.

Scope topology is a typed graph/DAG. Structural/contextual edges can contribute inherited context; associative edges establish relevance without unconditional inheritance. A client Connection may be structurally upstream of its Projects while a vendor Connection remains lateral/associative.

Planning work therefore needs explicit scope creation/editing, ownership routing, authority policy, typed graph edges, and context-resolution surfaces. See [ADR 0021](decisions/0021-typed-reliquary-scopes-and-connections.md) and [Reliquary and Phylactery memory scope plan](reliquary-phylactery-memory-scope-plan.md).

## Reliquary / Phylactery file-kind transition

Typed Reliquary `.rel` identity is implemented over the full former CVA model, and typed Phylactery `.phy` is implemented as the distinct User file kind. Product file identities are fixed by ADR 0020 and amended by ADR 0021:

- Reliquary is the typed non-user scope family → `<name>.<scope-type>.rel`;
- currently accepted Reliquary scope hints are `.org.rel`, `.prj.rel`, and `.con.rel`;
- Phylactery user-global Identity state → `.phy`.

The implementation preserves one shared low-level container/storage engine. New REL and PHY files encode and validate semantic file kind inside a 24-byte header; REL additionally carries Reliquary scope kind, while PHY requires the scope byte to be zero. Filename extensions are human-facing hints, not semantic authority. Existing 16-byte CVA headers are detected explicitly as legacy Project Reliquaries.

Remaining migration/product work should proceed in this order:

1. **Implemented:** explicit legacy `.cva` / earlier typed REL/PHY → current identified REL/PHY migration through semantic repack, preserving stable semantic IDs and deriving owner UUIDs from legacy workspace IDs when available;
2. update Warlock file creation/open/association UX to use typed REL and PHY files before retrieval/context assembly;
3. implement filename-hint versus authoritative internal-type mismatch warnings;
4. define explicit cross-file Memory/source export and lineage semantics between REL and PHY;
5. decide later whether the internal/back-compat `Cva` terminology should be removed.

Do not implement `.phy` as a project Reliquary with provenance fields merely made nullable. Shared mechanics and separate semantic validation are both required.

## Parallel Insomnia validation and integration

Routine prompt tuning on the 11-Episode adversarial fixture is complete and frozen. Future work is implementation/validation rather than continued fixture optimization:

1. **Implemented:** port the fixed-group metadata classification seam into authoritative extractor contract `v3-1` without allowing metadata to alter semantic groups;
2. **Implemented for the first boundary:** dedicated `user | project` persistence ownership classification after fixed groups/optional metadata and before wording, with Project as the conservative default. Ownership remains separate from semantic authority and does not alter candidate identity;
3. **Implemented for User routing:** explicit REL → PHY Memory routing, source-independent PHY publication, owner-qualified `MemoryRef` receipts, PHY-first crash recovery, finite-drain/CLI routing, and long-lived RuntimeHost attachment/vector backfill. Rich cross-file source lineage/export permission remains future work; Organization/Connection learned-state routing must also remain separate from authorization/governance;
4. run the full 66-Episode gold-v3 corpus as milestone confirmation, not as another prompt-tuning loop;
5. recalibrate worker concurrency and model/reasoning cost for the selected production semantic/metadata/ownership/wording model mix;
6. treat further semantic-quality work as a new capability boundary only when measured production failures justify it. Candidate escalation paths are documented in [Insomnia semantic validation — Future reliability architecture options](insomnia-semantic-validation-2026-08-24.md#future-reliability-architecture-options): targeted verifier/repair, selective multi-sample voting, deterministic clause-candidate preprocessing, ambiguity routing, a separate supersession resolver, provenance-specific verification, or a stronger/fine-tuned selector.

Do not resume benchmark-specific prompt squeezing or add a general semantic review/rewrite pass merely to chase stochastic misses on the tuning fixture. Any reliability architecture should be triggered by production-observed failure classes and should concentrate extra inference on ambiguous/high-risk cases rather than multiplying every Insomnia call by default.

## Later semantic layers

### Echo

Add Echo as the source-scoped historical reasoning owner defined by [ADR 0014](decisions/0014-echo-historical-reasoning-traces.md). Keep it cold by default, non-authoritative, and source-scoped rather than globally searchable.

### Dream over Graph

Initial Dream semantic validation is complete across retrieval/inference and controlled semantic-state writes. The first Ox Alpha/OpenRouter fixture found all 13 selected related Memory pairs within the default top 12, classified all 13 as non-none, independently accepted all 13 proposals, returned `none` for all 4 unrelated negatives, and scored 9/9 on the synthetic relation/direction contract. A second write-enabled fixture passed temporal-only retrieval with semantic/lexical lanes disabled, verified supersession publication/lifecycle projection, chronological duplicate-chain rewiring, successful `extracted → knowledge`, replay idempotency, and reopen recovery. The resulting conservative `knowledge → canonical` policy is implemented: persisted direct/correction authority can establish selected current-state classes immediately, independent duplicate corroboration requires distinct user authority anchors, and a unique verified successor inherits canonical status. Dream is also integrated into the shared `InteractionRuntime` background coordinator after Insomnia/vector work, with no separate Dream queue or process. No observed case currently justifies bounded reconsideration machinery; host-owned repeated wakeup/backoff, reconsideration, model-enriched temporal interpretation, or a derived temporal acceleration index remain future or measurement-driven.

The remaining redesign and implementation sequence are owned by [Dream implementation plan](dream-implementation-plan.md). That plan also records the indexed duplicate predecessor-chain design, implemented verification policy, and source-turn-timestamp-based temporal determinism.

### Memory-web organization

Owner-local derived Community snapshots use the implemented graph-local Leiden scan-and-merge reduction, and owner-local Memory retrieval now implements the validated Community-routing primitive described in [ADR 0026](decisions/0026-community-routed-memory-retrieval.md). Both REL and PHY can build a reusable transient `MemoryRetrievalIndex`; the default derives four sub-centroids per current Community, routes to four Communities plus the unclustered residual lane, exact-scores admitted Memories, selects four seeds, and traverses by Graph depth with Community locality only inside equal depth. An explicit/global fallback path remains available.

Remaining memory-web work is integration and measurement rather than another routing-quality gate:

- attach reusable `MemoryRetrievalIndex` lifecycle to Ego/runtime so repeated queries cache the derived index and deterministically rebuild on stale Memory/Graph/Community watermarks;
- compose owner-local REL and PHY retrieval above these primitives without introducing cross-owner Dream/Graph authority;
- measure release-mode production latency and integration behaviour against the explicit `GlobalExact` reference path while preserving an easy rollback during rollout;
- keep K=3 only as a possible explicit cost-biased mode if production economics justify exposing it; K=2 is not supported as a general default by the five-fold validation;
- consider split/merge lineage or continuity-preserving IDs only if a concrete product need appears;
- consider incremental invalidation/reduction reuse only if measured scan-and-merge cost becomes material; and
- keep human-facing community naming as Warlock presentation metadata rather than semantic Memory-Web authority.

### Ego

Add active context synthesis only after the shared runtime, Memory retrieval, and graph/lifecycle semantics are stable enough to provide trustworthy inputs.

## Storage, scale, and historical recovery

Reliquary/Phylactery storage remains purpose-built. Project-file history is delegated to the project repository (Lore by default, Git when explicitly owned by the user). Keep physical and retrieval work measurement-driven behind existing semantic boundaries.

Keep these separate and measurement-driven:

- mapped/segmented vector scanning and ANN acceleration;
- persistent lexical acceleration only if reopen/query measurements justify it;
- quantized searchable representations;
- explicit vector-generation retirement;
- REL/PHY packing, compaction, retention, and recovery where measured semantic-store needs justify them; and
- derived-state rebuild/invalidation policy for semantic owners.

The older whole-CVA versioning/rollback plan remains useful as a record of requirements and unresolved semantics, but project-file/history requirements should now be reassessed against the universal Lore/Git project repository. REL/PHY historical requirements should be retained only where they serve semantic/database recovery rather than duplicate project VCS. See [Versioning, historical cuts, and rollback](version-history-plan.md).

## Product acceptance gates

### Usable local product

A non-developer can create/open a Warlock workspace backed by a user-owned CVA, converse through Warlock, add/view/export files, browse conversations and durable knowledge, and understand basic provenance without CLI use.

### Interoperable product

The same CVA can receive equivalent normalized interaction history from the Warlock-native interaction path and at least one external agent protocol adapter without protocol-specific semantic records.

### Durable live product

Long-running capture, background processing, reconnect, crash/restart, and provider failures have explicit tested behavior and do not silently lose acknowledged source events.

### Expandable semantic product

New semantic owners remain purpose-built, use stable cross-owner IDs, and do not require a generalized CVA root/dependency framework.

## Open decisions

- storage and explicit management surfaces for Organization, Project, and Connection scopes, including ownership routing, authorization/governance, typed graph edges, hierarchy/composition, and runtime rendering;
- Whether later Phylactery capabilities justify additional purpose-built owners beyond the implemented Memories/Graph/Packed Vectors/Memory Vectors/Compatibility Profiles core, and how future user-Memory lexical retrieval should be represented without importing Archive semantics.
- Whether `CVA` remains only as an internal generic-container/back-compat term now that explicit legacy migration is implemented.
- The exact ownership boundaries among user-global Phylactery, Organization Reliquary, Project Reliquary, and Connection Reliquary learned state.
- How the implemented dedicated `user | project` ownership classifier expands to Organization/Connection learned state without conflating ownership with authorization/governance.
- Richer Reliquary policy for permitting/denying user-Memory export and optional source/lineage export into Phylactery; the current routing slice strips REL-local provenance and records only the resulting owner-qualified Memory reference.
- Normalized interaction vocabulary for tool/session/artifact events beyond completed user/agent turns.
- How REL/PHY semantic databases should synchronize across devices now that project-file history is owned separately by Lore/Git, including whether ordinary cloud-drive conflicted-copy transport remains sufficient or a later coordination layer is justified.
- Which Reliquary mutations are safe to expose as direct user actions once project-file history no longer depends on whole-REL historical assumptions.
- Adapter failure policy and privacy controls for automatic capture.
- Exact repository-backed `ProjectFileRef` / revision-reference schema shared by transcript, provenance, and artifact records across Lore/Git.
- File path/tree ownership and rename/move identity semantics within the universal project repository.
- Artifact provenance vocabulary across uploaded, generated, imported, and provider-managed artifacts.
- Archive checkpoint representation, packing/compression choices, and retention policy.
- Whole-CVA restore/timeline terminology and retention semantics.

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

## Notes

This file is future-only by policy. When a roadmap item ships, remove it from this document and document the resulting behavior in the current-state owners instead.
