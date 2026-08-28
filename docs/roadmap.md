# Roadmap

Parent index: [Documentation index](INDEX.md)

## Purpose

This document owns future implementation work for Reliquary. Completed behavior does not belong here; current behavior is documented in [Architecture](architecture.md), [Rust API](api.md), [Repo-local CLI](cli.md), and [Current limitations](current-limitations.md).

## Overview

Future work is organized around productization first, with intelligence quality and storage/history work proceeding in parallel where they do not block the usable product surface. New semantic owners remain purpose-built and shared mechanics are introduced only where concrete owners or runtime requirements justify them.

## Product direction

Reliquary is moving from a storage/retrieval substrate toward the durable workspace/context layer of the broader Warlock product.

One Reliquary REL maps to one non-user Warlock scope/workspace; the current Warlock host uses Project RELs. Warlock owns the native application surface and long-lived application orchestration; Reliquary remains the Rust semantic/storage subsystem linked into that application core. External agent protocols such as ACP remain interoperability adapters into the same normalized interaction seam and are not canonical storage schemas. See [ADR 0016](decisions/0016-native-product-surface-and-shared-interaction-runtime.md) and [ADR 0017](decisions/0017-cva-workspace-and-warlock-host-application.md).

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
- cloud-backed conflicted-copy detection/reconciliation through Reliquary rather than a Warlock-owned sync service.

`Cva::compare` verifies shared durable owner ID and classifies identical, one-side-ahead, or physically diverged histories. `Cva::reconcile` handles true divergence by semantic replay into a fresh validated CVA when the candidate contributes new state. If a physically divergent candidate is already semantically absorbed, `canonical_change_required` is false and reconciliation preserves an exact copy of the canonical CVA instead of persisting reconciliation receipts or retiring vector state. Real changed merges replay Archive source state, Files, Branches, immutable Episodes/Fragments, Memory revisions, durable Insomnia completion receipts, file-to-Memory links, and compatibility profiles; stale packed/vector bindings and Vector Generations are then intentionally retired and `vector_rebuild_required` exposes the need for verified re-embedding. `Cva::reconcile_and_promote` skips replacement for no-ops and otherwise provides canonical fingerprint guarding, a synced recovery copy, atomic replacement, post-replacement validation/recovery, and temporary-artifact cleanup. Known semantic merge collisions surface as typed `CvaReconcileConflict` values for host presentation rather than raw Archive/Memory/Insomnia errors. Remaining work is provider-level conflicted-copy discovery, host-triggered vector rebuild, Warlock-side conflict presentation/resolution, and realistic repeated multi-device/provider fixtures. See [ADR 0019](decisions/0019-cloud-backed-cva-reconciliation.md).

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

The owner-local full-Graph Leiden baseline is implemented as derived Community snapshots. Remaining memory-web work is:

- affected-region scan-and-merge so Graph changes do not require full reclustering;
- explicit split/merge lineage and continuity-preserving community identity where justified;
- community-aware Graph traversal/pruning with measured retrieval/traversal benefit; and
- Warlock presentation metadata for editable human-facing community names.

### Ego

Add active context synthesis only after the shared runtime, Memory retrieval, and graph/lifecycle semantics are stable enough to provide trustworthy inputs.

## Storage, scale, and historical recovery

Keep these measurement-driven and independent from product-surface work:

- Archive checkpoint representation/cadence;
- bounded packing/compression;
- mapped/segmented vector scanning and ANN acceleration;
- persistent lexical acceleration only if reopen/query measurements justify it;
- quantized searchable representations;
- explicit vector-generation retirement;
- whole-CVA historical views and restore-and-continue;
- retention, reachability, compaction, and vacuum;
- concurrent append/version reservation;
- owner-explicit reconciliation support for each later persisted semantic owner as it lands; Graph is now covered, while derived indexes such as Dream's duplicate index rebuild instead of becoming CVA synchronization state.

Whole-CVA historical recovery has its own future-only plan in [Versioning, historical cuts, and rollback](version-history-plan.md).

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
- Whether any future headless/remote product mode justifies adding a service/IPC boundary around the in-process Rust integration.
- Which CVA mutations are safe to expose as direct user actions before whole-history retention semantics exist.
- Adapter failure policy and privacy controls for automatic capture.
- File path/tree ownership and rename/move identity semantics.
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

## Notes

This file is future-only by policy. When a roadmap item ships, remove it from this document and document the resulting behavior in the current-state owners instead.
