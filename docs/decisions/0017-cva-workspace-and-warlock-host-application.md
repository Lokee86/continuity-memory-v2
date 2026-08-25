# ADR 0017: CVA workspace and Warlock host application

## Status

Accepted — 2026-08-24. Supersedes the standalone native-Continuity-UI product direction in ADR 0016; ADR 0016 remains authoritative for the shared transport-neutral interaction/runtime seam.

## Purpose

Define Continuity's role inside the broader Warlock product and settle the user-facing workspace boundary.

## Overview

One CVA is one durable Warlock workspace. Warlock is the user-facing application and runtime host. Continuity remains a Rust library/subsystem that owns CVA semantics, source history, files, Memories, provenance, retrieval, and its purpose-built semantic owners.

Warlock should link Continuity directly into its Rust application core when practical. A separate Continuity daemon, mandatory IPC layer, or standalone Continuity desktop application is not required.

## Context

ADR 0016 established a transport-neutral live interaction/runtime seam so native and external interaction paths could converge before semantic storage. It assumed Continuity itself would own the primary native product interface.

Subsequent Warlock design established a broader product model: Warlock manages multiple kinds of work, while Continuity supplies the durable context/state substrate. A separate Continuity application would duplicate workspace management and split one product into competing shells.

The workspace boundary also provides useful context isolation. Development, business, construction, research, and other contexts can remain separate by default while still being deliberately related later.

## Decision

### One CVA is one workspace

A CVA is the durable workspace artifact rather than merely an archive opened by another workspace manifest.

The minimum workspace identity is explicit and purpose-built:

```text
WorkspaceMetadata
├── stable workspace ID
├── display name
└── workspace type
```

Workspace metadata must not become a generic property bag or generalized semantic root.

### Workspace type composes the host experience

`WorkspaceType` identifies the domain/capability composition expected by the host application.

Examples include `software`, `construction`, and future types. Workspace type does not transfer semantic ownership into Continuity. Software repositories, accounting systems, email systems, domain adapters, and other resources keep their own authority.

### External resources can remain external

The CVA does not need to physically absorb every resource used by the workspace.

A workspace may contain or reference:

- embedded user files and generated artifacts;
- external software repositories;
- email messages/threads through provider integrations;
- accounting entities through systems such as QuickBooks;
- cloud storage, calendars, and other APIs;
- derived or synchronized evidence from those systems.

The external system remains authoritative for its native records unless an explicit import creates a separate durable source artifact. The CVA may retain stable references, bounded cached evidence, provenance, relationships, and derived Memories needed to make that information part of the workspace context.

### Warlock is the native product surface

Continuity does not require a standalone GUI. Warlock owns ordinary workspace creation/open/close, navigation, presentation, capability composition, and user interaction.

Conceptually:

```text
Warlock application
├── TypeScript presentation
└── Rust application core
    ├── workspace management
    ├── Continuity library
    │   └── Workspace.cva
    ├── capabilities/tools
    └── external integrations
```

The presentation layer must not parse CVAs, mutate semantic owners directly, own credentials, or implement Continuity lifecycle rules.

### Direct library integration is preferred

Continuity should be linked directly into the Warlock Rust core when the host and Continuity run in the same process. This avoids an unnecessary local daemon/HTTP/IPC boundary.

A service boundary remains valid for future headless, remote, multi-process, or interoperability use if a concrete requirement appears. It is not the default product topology.

### Shared interaction runtime remains transport-neutral

The `InteractionRuntime` direction from ADR 0016 remains valid. Warlock-native interaction, ACP, imports, and future adapters normalize into the same source semantics before Archive publication.

Warlock may provide the long-lived scheduling/execution loop around Continuity's library-level runtime without becoming the semantic owner of Archive, Memories, Insomnia, Dream, Echo, or Ego.

### Context isolation defaults to workspace scope

Retrieval and active context should default to the current CVA/workspace. Cross-workspace discovery or linking must be explicit rather than silently pooling all user state into one retrieval universe.

This permits, for example, development context and business context for the same broader endeavour to remain separate while still supporting future explicit relationships between workspaces.

## Consequences

- Continuity has one clear product role: durable workspace/context substrate.
- Warlock has one clear product role: management application and capability host.
- No second workspace manifest is required unless future requirements prove the CVA cannot own necessary workspace identity.
- Workspace types can vary widely without changing CVA ownership rules.
- External APIs can participate in a workspace without turning the CVA into a replacement ERP, email server, or source-control system.
- Software can be the first proving workspace type while construction and other domains remain future compositions.

## Initial implementation

1. **Implemented:** add purpose-built workspace metadata to the CVA: ID, name, and type.
2. **Implemented:** expose create/open/read plus one-time initialization through the Rust library API; rename/type-change lifecycle is intentionally deferred.
3. Keep repository/API/resource bindings outside the first metadata slice unless a concrete host workflow requires them.
4. Let Warlock open one CVA and use its type to select the workspace presentation/capabilities.
5. Prove durable conversation/file state through close/reopen before adding external integrations.

## Rejected alternatives

### Standalone Continuity desktop application

Rejected. It duplicates the Warlock shell and fragments workspace ownership.

### Separate Warlock workspace manifest beside the CVA

Rejected as the default. It creates two competing durable workspace identities without a demonstrated need.

### Mandatory Continuity daemon

Rejected. In-process Rust integration is simpler when Warlock is the local application host.

### Embed every external system record

Rejected. External systems can remain authoritative while contributing references/evidence to the workspace.

### One global user context

Rejected. Workspace isolation is valuable for relevance, provenance, privacy boundaries, and domain separation.

## Verification

Focused tests now prove that workspace metadata survives reopen, initializes only once, validates bounded fields, and does not advance Archive, Memory, or Vector Generation semantic clocks. Future tests should additionally prove:

- host-level one-CVA/one-workspace identity remains stable through Warlock lifecycle;
- future display-name/type changes do not rewrite Archive or Memory authority;
- Warlock-native and adapter interactions normalize to equivalent source semantics;
- external references cannot silently become user/source authority;
- retrieval remains workspace-scoped unless an explicit cross-workspace operation is requested.

## Related docs

- [Architecture](../architecture.md)
- [Roadmap](../roadmap.md)
- [ADR 0015](0015-acp-inline-interaction-stream.md)
- [ADR 0016](0016-native-product-surface-and-shared-interaction-runtime.md)

## Notes

This decision defines product/application composition. It does not make Warlock the owner of Continuity semantic data or require Continuity to depend on Warlock.