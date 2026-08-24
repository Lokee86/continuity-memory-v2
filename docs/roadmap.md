# Roadmap

Parent index: [Documentation index](INDEX.md)

## Purpose

This document owns future implementation work for Continuity. Completed behavior does not belong here; current behavior is documented in [Architecture](architecture.md), [Rust API](api.md), [Repo-local CLI](cli.md), and [Current limitations](current-limitations.md).

## Overview

Future work is organized around productization first, with intelligence quality and storage/history work proceeding in parallel where they do not block the usable product surface. New semantic owners remain purpose-built and shared mechanics are introduced only where concrete owners or runtime requirements justify them.

## Product direction

Continuity is moving from a storage/retrieval substrate toward a usable commercial product for both technical and non-technical users.

The primary user experience will be a native Continuity interface backed by a shared long-lived runtime. External agent protocols such as ACP are interoperability adapters into that runtime, not prerequisites for using the product and not canonical storage schemas. See [ADR 0016](decisions/0016-native-product-surface-and-shared-interaction-runtime.md).

## Near-term productization sequence

### 1. Shared live interaction runtime

Build one long-lived runtime that owns live session execution and normalizes incoming interaction events before they reach semantic storage owners.

Required behavior:

- accept individual live user/agent interaction events rather than requiring batch import;
- preserve stable session/conversation identity across reconnect/resume;
- normalize completed messages, attachments/artifacts, and supported tool/session events without making any transport protocol semantically authoritative;
- assemble streamed messages into one documented durable publication boundary;
- schedule inactivity/size-driven Episode work continuously;
- run Memory/vector work continuously in the background;
- cache verified model/embedding capabilities safely;
- expose the narrow explicit memory-control operations needed by hosts;
- define heartbeat, cancellation, retry, and shutdown behavior for long-running inference work.

### 2. CVA management API

Expose coherent user-facing administration without introducing a generalized database abstraction.

The management surface should compose explicit owner operations for:

- conversation/session inventory and inspection;
- file inventory, import, export, source provenance, and later organization;
- Memory inventory, provenance, lifecycle inspection, and permitted manual lifecycle actions;
- health, verification, statistics, and diagnostics;
- selective import/export of user-owned state;
- safe unlink/removal operations only after retention/history semantics are defined.

The management layer must remain an application/service surface over concrete owners, not a new semantic owner or generic mutable object store.

### 3. Native Continuity interface

Build the first direct product surface over the shared runtime and management API.

The initial interface should make a CVA useful without exposing storage internals:

- workspace/project selection;
- conversational agent surface;
- conversation/history browser;
- file browser;
- Memory/knowledge browser;
- provenance/history inspection;
- agent/model selection and configuration;
- basic health/status visibility.

The interface should be useful to a non-developer without requiring ACP, MCP, a terminal, an IDE, or knowledge of CVA internals.

### 4. ACP interoperability adapter

Implement ACP as a first-class external agent/client adapter over the same live runtime used by the native interface.

Required work includes:

- map ACP session/message identity onto the normalized interaction/session contract;
- capture supported observable messages, attachments, tool events, and surfaced reasoning without treating ACP metadata as semantic authority;
- define streamed-message assembly and acknowledgement durability;
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

## Parallel intelligence-quality work

Continue Insomnia quality work independently from product-surface construction:

1. integrate the two-pass authority/disposition architecture into the authoritative runtime so pass 1 owns disposition, authority, lifecycle, provenance, and other selected semantic metadata while pass 2 is restricted to faithful wording/consolidation;
2. verify the integrated runtime against the focused tuning fixture, including mixed-turn omission/retention and provenance ownership boundaries;
3. run the large corpus only as a milestone confirmation after the integrated path is stable;
4. tune synthesis-model/reasoning cost and throughput only after the ownership split is verified;
5. recalibrate worker concurrency for the selected production model mix rather than carrying forward an older optimum by assumption.

Do not add a general third semantic-review pass or broader retrieval unless a measured failure demonstrates a distinct need.

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

A non-developer can create/open a user-owned CVA, converse through the native interface, add/view/export files, browse conversations and durable knowledge, and understand basic provenance without CLI use.

### Interoperable product

The same CVA can receive equivalent normalized interaction history from the native interface and at least one external agent protocol adapter without protocol-specific semantic records.

### Durable live product

Long-running capture, background processing, reconnect, crash/restart, and provider failures have explicit tested behavior and do not silently lose acknowledged source events.

### Expandable semantic product

New semantic owners remain purpose-built, use stable cross-owner IDs, and do not require a generalized CVA root/dependency framework.

## Open decisions

- User-facing workspace/project model and whether one CVA maps one-to-one to that concept.
- Exact normalized interaction/session event vocabulary above Archive ingestion.
- Native UI/runtime process topology and local IPC boundary.
- Which CVA mutations are safe to expose as direct user actions before whole-history retention semantics exist.
- Streaming acknowledgement point: before or after durable source publication.
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

## Notes

This file is future-only by policy. When a roadmap item ships, remove it from this document and document the resulting behavior in the current-state owners instead.
