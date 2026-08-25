# Roadmap

Parent index: [Documentation index](INDEX.md)

## Purpose

This document owns future implementation work for Reliquary. Completed behavior does not belong here; current behavior is documented in [Architecture](architecture.md), [Rust API](api.md), [Repo-local CLI](cli.md), and [Current limitations](current-limitations.md).

## Overview

Future work is organized around productization first, with intelligence quality and storage/history work proceeding in parallel where they do not block the usable product surface. New semantic owners remain purpose-built and shared mechanics are introduced only where concrete owners or runtime requirements justify them.

## Product direction

Reliquary is moving from a storage/retrieval substrate toward the durable workspace/context layer of the broader Warlock product.

One CVA maps to one Warlock workspace. Warlock owns the native application surface and long-lived application orchestration; Reliquary remains the Rust semantic/storage subsystem linked into that application core. External agent protocols such as ACP remain interoperability adapters into the same normalized interaction seam and are not canonical storage schemas. See [ADR 0016](decisions/0016-native-product-surface-and-shared-interaction-runtime.md) and [ADR 0017](decisions/0017-cva-workspace-and-warlock-host-application.md).

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

### 2. Workspace/CVA application API

Expose the concrete Rust operations Warlock needs to manage one CVA-backed workspace without introducing a generalized database abstraction or mandatory service layer.

The application surface should compose explicit owner operations for:

- broader conversation/session management beyond the implemented derived conversation summaries and exact-leaf transcript reads;
- file inventory, import, export, source provenance, and later organization;
- Memory inventory, provenance, lifecycle inspection, and permitted manual lifecycle actions;
- health, verification, statistics, and diagnostics;
- selective import/export of user-owned state;
- safe unlink/removal operations only after retention/history semantics are defined.

The management layer must remain an application/service surface over concrete owners, not a new semantic owner or generic mutable object store.

### 3. Warlock host integration

Reliquary is now linked directly into the Warlock v2 Rust application core. One CVA is the durable workspace file, and Warlock currently implements create/open/close/reopen plus durable conversation list/start/resume/user-turn/reopen through the public Reliquary runtime seam.

Remaining host integration work includes:

- provider/agent execution and durable assistant turns;
- interaction attachments and explicit multi-leaf branch selection;
- file/artifact access;
- Memory/knowledge access;
- provenance/history inspection;
- model/agent configuration seams required by the host;
- background Episode/Insomnia execution; and
- basic health/status visibility.

The TypeScript presentation layer consumes Warlock application commands/state and must not parse CVAs or implement Reliquary lifecycle semantics directly.

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

## Parallel Insomnia validation and integration

Routine prompt tuning on the 11-Episode adversarial fixture is complete and frozen. Future work is implementation/validation rather than continued fixture optimization:

1. port the validated dedicated metadata-classification ownership split into the authoritative runtime without allowing metadata to alter frozen semantic groups;
2. design and fixture-test the `user | project` persistence-scope boundary described in [Reliquary and Phylactery memory scope plan](reliquary-phylactery-memory-scope-plan.md). Do not assume it belongs in the existing metadata pass: a dedicated narrow scope-classification pass may be the cleaner architecture and must be compared explicitly;
3. define the separate **Phylactery** CVA-type contract, including optional rather than mandatory source provenance and Reliquary-controlled Memory/source export policy;
4. run the full 66-Episode gold-v3 corpus as milestone confirmation, not as another prompt-tuning loop;
5. recalibrate worker concurrency and model/reasoning cost for the selected production semantic/metadata/scope/wording model mix;
6. treat further semantic-quality work as a new capability boundary only when measured production failures justify it. Candidate escalation paths are documented in [Insomnia semantic validation — Future reliability architecture options](insomnia-semantic-validation-2026-08-24.md#future-reliability-architecture-options): targeted verifier/repair, selective multi-sample voting, deterministic clause-candidate preprocessing, ambiguity routing, a separate supersession resolver, provenance-specific verification, or a stronger/fine-tuned selector.

Do not resume benchmark-specific prompt squeezing or add a general semantic review/rewrite pass merely to chase stochastic misses on the tuning fixture. Any reliability architecture should be triggered by production-observed failure classes and should concentrate extra inference on ambiguous/high-risk cases rather than multiplying every Insomnia call by default.

## Later semantic layers

### Echo

Add Echo as the source-scoped historical reasoning owner defined by [ADR 0014](decisions/0014-echo-historical-reasoning-traces.md). Keep it cold by default, non-authoritative, and source-scoped rather than globally searchable.

### Graph and Dream

Add Graph as its own semantic owner and reconnect Dream with pair-oriented relationship evaluation. Processing direction must not determine semantic edge direction. Rebuild lifecycle handling around `extracted → knowledge → canonical`, with `archived` as retained inactive history for superseded and non-representative duplicate Memories.

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
- concurrent append/version reservation.

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

- Phylactery storage shape and the exact boundary between user-global Phylactery Memory and project-local Reliquary Memory.
- Whether `user | project` scope belongs in the existing metadata pass or a dedicated scope-classification pass; if dedicated, its ordering relative to metadata and synthesis.
- Reliquary policy for exporting user Memories and optionally their source/provenance into Phylactery.
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

## Notes

This file is future-only by policy. When a roadmap item ships, remove it from this document and document the resulting behavior in the current-state owners instead.
