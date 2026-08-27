# Reliquary and Phylactery memory scope plan

Parent index: [Documentation index](INDEX.md)

## Purpose

This document defines the planned durable ownership boundaries, file kinds, scope graph, context-composition rules, and validation requirements for Reliquary and Phylactery memory scopes.

## Overview

Durable state is partitioned by semantic owner rather than stored in one global memory pool. User-global state belongs to Phylactery; Organization, Project, and Connection state belong to typed Reliquary scopes. Cross-scope context is composed through explicit typed relationships and policy without transferring ownership merely because one scope references or physically contains another.

## Status

Provisional implementation plan. **Reliquary** and **Phylactery** are accepted names under [ADR 0018](decisions/0018-reliquary-and-phylactery-naming.md). [ADR 0020](decisions/0020-reliquary-and-phylactery-file-kinds.md) establishes the `.rel`/`.phy` product file split, and [ADR 0021](decisions/0021-typed-reliquary-scopes-and-connections.md) amends Reliquary from a project-only concept into a typed family of non-user durable scopes.

Typed Reliquary `.rel` files are implemented over the existing full CVA storage model, including authoritative Organization/Project/Connection scope identity and explicit legacy `.cva` detection as Project Reliquary. Typed Phylactery `.phy` is also implemented as the distinct User file kind with a source-independent Memory/Graph/vector/profile owner set. Multi-scope persistence routing, cross-file provenance/export, and scope-graph context resolution are not yet implemented.

## Problem

A single omni-memory pool creates both retrieval pollution and state-ownership ambiguity. Durable state does not all belong to the same thing.

The currently identified state owners are:

- **User / Phylactery** — user-global Identity state that persists across work;
- **Organization / Reliquary** — company/organization state that persists across projects;
- **Project / Reliquary** — bounded work state specific to one project;
- **Connection / Reliquary** — durable state belonging to a relationship between parties/scopes.

These are ownership boundaries first. Retrieval and context composition are separate concerns.

A company policy does not become Project state because it was used while working on a project. A project delivery delay does not become Organization state because company personnel observed it. Supplier reliability history does not necessarily belong to either the supplier participant or the current project; it can belong to the Connection itself.

## Product file direction

### Phylactery

User-global state remains **Phylactery `.phy`**, a distinct semantic file kind rather than a Reliquary with fields removed.

Its implemented core is durable user Memory plus same-file Graph state, Packed Vectors, Memory Vectors, and Compatibility Profiles. Archive/history, Episodes, Files/attachments, Insomnia state, Archive Vectors, Vector Generations, interaction-stream checkpoints, and the current Archive-specific lexical index are not Phylactery owners.

A Phylactery Memory does not require project source turns or a live pointer to an originating Reliquary. Current direct publication requires the existing REL-local Episode/node/conversation provenance fields to be absent. Future source export may retain permitted provenance only through an explicit cross-file lineage/snapshot representation; source absence must remain valid.

### Reliquary family

Non-user durable scopes use the **Reliquary `.rel` family** over shared storage/container mechanics.

New human-facing files should use a typed double extension:

```text
<name>.<scope-type>.rel
```

Current accepted type abbreviations:

- Organization: `.org.rel`
- Project: `.prj.rel`
- Connection: `.con.rel`

Examples:

```text
acme.org.rel
tower-a.prj.rel
willyswidgets.con.rel
```

The final `.rel` identifies the Reliquary family. The penultimate extension is a human-visible scope hint. The internal file-kind/scope-type discriminator is authoritative; renaming a file must not change its semantic type.

Hierarchy is not encoded in filenames.

No Person Reliquary scope is currently established. It must not be added without an independent durable state-ownership requirement.

### Shared mechanics, distinct semantics

Reliquary and Phylactery should share low-level framing, checksums, immutable object storage, versioning, vector/graph primitives, recovery, compaction, and migration machinery where those mechanics are genuinely common.

Their semantic validation and permitted owners remain distinct.

Within Reliquary, Organization, Project, and Connection may also share many physical owners while retaining scope-specific validity and policy rules.

Existing `.cva` files remain legacy Project Reliquary data for migration purposes; new Reliquary creation uses typed `.rel` headers without removing any of the current storage owners.

## Ownership versus authority

Scope ownership does not imply that every kind of state may be inferred or mutated automatically.

This distinction is especially important for Organization and Connection scopes.

An Organization may own both:

- learned operational state/history; and
- explicitly governed policies, permissions, procedures, compliance rules, and institutional instructions.

A Connection may own both:

- learned relationship state/history; and
- explicitly governed contractual terms, negotiated exceptions, standing permissions, commitments, and operating instructions.

Insomnia/Dream must never convert repeated observed behaviour directly into authoritative policy, permission, contract terms, or governed instructions merely because the observation belongs to that scope.

The eventual schema must therefore preserve at least the conceptual distinction between **learned/synthesized state** and **explicitly governed state**. Mutation authority is a separate policy from scope ownership.

## Organization scope

Organization owns durable state belonging to the organization independently of individual projects.

In construction this includes, for example:

- day-to-day company operations;
- estimating and bidding practices;
- purchasing and supplier practices;
- scheduling and crew conventions;
- accounting/administrative conventions;
- approval paths;
- safety/security/compliance policy;
- institutional terminology and defaults;
- recurring operational history and learned company knowledge.

Project-specific implementation state remains outside Organization even when the project is owned by that organization.

Organization is therefore a genuine state scope, not merely a persistent instruction addendum.

## Project scope

Project owns durable state specific to one bounded body of work.

Project state includes project source/history, files, decisions, constraints, schedules, deliveries, issues, Memories, provenance, vectors, Graph state, owner-local Dream-derived lifecycle/relationship state, and later Ego-derived project context.

A Project may inherit/apply Organization and Connection context without transferring ownership of that inherited state into the Project.

## Connection scope

A Connection owns durable state about the relationship between parties/scopes.

The term **Connection** replaces `Relationship` in the scope vocabulary. This avoids `.rel.rel` filenames and reduces ambiguity with graph/database relationships.

Connections are unusual because they are often not neutral operational contexts. Most interactions with a client, vendor, consultant, partner, or personal contact concern some project, business matter, or event. Nevertheless, durable state exists that belongs to the relationship itself rather than to the surrounding matter.

Examples include:

- standing communication norms;
- relationship history across projects;
- recurring reliability patterns;
- account-level disputes or unresolved issues;
- negotiated expectations or exceptions;
- durable commitments;
- commercial terms where storage policy permits;
- personal/shared history in non-commercial connections.

### Connection classification

Connection should not be implemented as a rigid subclass hierarchy such as `VendorConnection`, `ClientConnection`, etc. One relationship can hold multiple roles, and roles can change.

Instead a Connection should contain:

- a broad, extensible `class`;
- its participants;
- directional participant roles;
- durable Connection state;
- typed graph edges to relevant scopes.

Provisional classes include:

- `commercial`
- `professional`
- `organizational/internal`
- `personal`

Provisional roles include:

- `client`
- `vendor`
- `supplier`
- `subcontractor`
- `general_contractor`
- `consultant`
- `partner`
- `employee`
- `employer`

These are examples, not a frozen ontology.

Roles are directional. The same two-party Connection can describe different roles from each participant's perspective, and a participant can hold more than one role.

## Scope graph

The scope topology is a **typed graph/DAG**, not a universal tree.

Some relationships are structural/contextual:

```text
Organization -> Project
Client Connection -> Project
```

Others are associative:

```text
Project -> Vendor Connection
Project -> Consultant Connection
```

A client Connection may legitimately act as hierarchy-like context above one or more projects. A vendor Connection may instead be a lateral participant used by many projects. Therefore hierarchy is a property of the specific typed edge/role configuration, not of `Connection` as a class.

At minimum, future graph design must distinguish:

- **structural/contextual edges** that can contribute default inherited context;
- **associative edges** that establish relevance without unconditional inheritance.

The final edge vocabulary and traversal rules remain open.

## Context composition

Normal context must be composed from applicable scopes rather than by searching every Reliquary.

Conceptually, work inside a project may assemble:

```text
current interaction
+ relevant Phylactery/User state
+ relevant Project state
+ applicable Organization state
+ applicable structural Connection state
+ task-relevant associated Connection state
```

Inactive/unrelated projects and Connections do not automatically participate.

Context composition must preserve three separate questions:

1. **Ownership:** what scope durably owns this state?
2. **Access:** is the current actor/runtime permitted to inspect it?
3. **Retrieval/composition:** should it participate in this execution context now?

Do not collapse those into a single namespace/filter decision.

## Nested scope discovery and evidence extraction

Reliquary and Phylactery files may physically appear inside directories observed by another Reliquary. They must be recognized as independent scope boundaries rather than recursively ingested as ordinary source artifacts.

The default ingestion behaviour is:

```text
other .rel/.phy encountered
        ↓
recognize scope/container
        ↓
do not ingest its container bytes as ordinary evidence
        ↓
resolve permitted scope identity/edge
        ↓
query through scope interface when evidence/state is needed
```

This is not equivalent to ignoring subordinate scopes. A parent, ancestor, or associated scope may deliberately consume permitted evidence or state from another scope.

The cross-scope operations should remain distinct:

- **discover/reference** — identify the scope and relationship;
- **read/extract** — inspect permitted evidence/state while ownership remains with the source;
- **copy/export** — preserve selected evidence or derived state in the destination scope when policy requires independent durability.

Example:

```text
Tower A Project
  ├── vendor promise evidence
  └── late delivery evidence
          ↓ permitted extraction
Willy's Widgets Connection
  └── learned reliability Memory
      provenance -> Tower A evidence
```

Depending on export policy, the destination may retain only a stable source reference, a bounded evidence snapshot, or a full permitted evidence copy. It must not recursively embed the source `.rel`/`.phy` container.

The invariant is:

> **Scopes may consume permitted evidence from other scopes, but they never acquire another scope merely by containing its file. Cross-scope state movement is semantic extraction/publication, not recursive repository ingestion.**

This keeps physical repositories independent while allowing the logical scope graph to compose evidence and state across Organization, Project, Connection, and User boundaries.

## Interaction/event boundary

Ordinary meetings, messages, conversations, and interactions are primarily durable evidence/history, not automatically independent scopes.

One interaction may feed several owners:

```text
interaction
├── Project Memory
├── Organization Memory
└── Connection Memory
```

This is preferable to creating an Interaction scope for every event.

A new scope kind should be created only when the thing has durable state and lifecycle that cannot be assigned correctly to surrounding scopes.

## Insomnia scope classification

The first persistence classifier is now implemented as `user | project` under ADR 0023. It is deliberately conservative and remains only the first boundary, not the complete long-term ownership model.

The eventual classifier/router must support the real durable owners that are authorized for learned state:

- `user`
- `project`
- `organization`
- `connection`

Organization/Connection classification must not imply permission to create governed policies, permissions, contractual terms, or standing instructions. Scope classification answers **where a proposition belongs**; authority/policy answers **whether and how it may be written**.

Atomicity matters. A mixed interaction should become separate durable propositions when its content belongs to different owners.

Example:

- `Tower A joists from Willy's Widgets will arrive Friday` -> Project;
- `Willy's Widgets has repeatedly missed promised delivery dates` -> Connection.

## Pass-boundary direction

The implemented `v3-1` extraction/routing sequence is:

`semantic authority/disposition -> fixed groups -> optional metadata classification -> optional ownership classification -> wording -> owner publication`

Persistence ownership is a separately testable classification/routing stage. For the current User/Project slice it runs after groups are fixed (and after metadata when configured) but before wording, and may change only the destination owner. It cannot change the durable proposition, authority/provenance, category/type/lifecycle, group membership, or candidate identity.

A later governance/export-policy stage may still be required before learned Organization/Connection publication. That is separate from the now-resolved placement of User/Project ownership classification.

The classifier should be evaluated independently from authority policy. A correct Organization/Connection ownership prediction can still result in `do not publish` or `require explicit authorization` for the proposed state type.

## Publication and export boundary

Scope ownership, export permission, and source-export permission are separate decisions.

A User Memory extracted from a Project may classify correctly as Phylactery-owned while project policy forbids exporting it. Likewise, Organization or Connection state may be semantically owned by those scopes while confidentiality or authorization policy blocks publication.

The current implementation has no general export-policy engine. Supplying an explicit PHY routing target enables User publication; Project-only paths do not run the ownership classifier. A routed User Memory is made source-independent by stripping REL-local provenance while retaining resolved `source_time_ns` semantic chronology; the REL completion records `MemoryRef { owner_id, memory_id }` to the resulting PHY object.

A richer future policy may still distinguish `memory_export = allow | deny` and `source_export = allow | deny`, especially for Organization/Connection or confidential Project material. A destination Memory must not require a live cross-file source pointer unless that source dependency is itself an explicit product contract.

## Retrieval implication

Retrieval is compositional rather than an omni-memory search.

The resolver should:

- identify the active scope(s);
- resolve applicable structural/contextual scope edges;
- identify task-relevant associated scopes where needed;
- query each permitted owner independently;
- query Phylactery for relevant user-global state;
- combine/rank the resulting evidence within a context budget.

This allows individual Projects, Organizations, and Connections to grow independently without forcing every memory into one global candidate set.

## Validation requirements

Scope fixtures should eventually include:

- obvious User-only Memories;
- obvious Project-only Memories;
- obvious Organization-only learned operational Memories;
- obvious Connection-only learned relational Memories;
- mixed interactions that must split across multiple owners;
- project-specific preferences that must not leak to Phylactery or Organization;
- organization-wide practices observed during one project that should route to Organization only when authority policy permits learned operational state;
- recurring vendor/client history that belongs to Connection rather than Project;
- client Connections that structurally contextualize Projects;
- vendor Connections that remain associative rather than inherited;
- NDA/confidential cases where ownership is clear but publication/export is forbidden;
- source-export-denied cases where destination Memory can remain valid without source content;
- inactive/mothballed scope cases verifying unrelated state does not enter ordinary retrieval.

Evaluate ownership accuracy separately from extraction coverage, semantic metadata, wording quality, authorization, export-policy enforcement, and context-resolution accuracy.

## Open decisions

- whether later Phylactery capabilities justify additional purpose-built owners beyond the implemented Memory/Graph/vector/profile core;
- exact cross-file source-lineage/export representation for permitted Phylactery provenance;
- exact Organization, Project, and Connection owner sets;
- whether bare `.rel` remains creatable or only supported for compatibility/migration;
- representation and validation of learned versus governed state inside Organization and Connection;
- automatic publication/authorization policy for Organization and Connection learned state;
- Connection class and directional role vocabulary;
- temporal role changes;
- concrete scope-edge vocabulary and structural/associative traversal semantics;
- context-resolution algorithm and ranking across several active scopes;
- exact ownership-classification pass placement;
- source-copy versus lineage representation across scope boundaries;
- memory/source export policy representation;
- correction, supersession, deduplication, and retirement across each scope kind;
- cross-owner context composition belongs to retrieval/Ego rather than Dream; Dream remains strictly owner-local and does not federate REL/PHY candidates or persist cross-file Graph relationships;
- Ego/context assembly budget and conflict resolution across inherited and associated scopes;

## Related docs

- [ADR 0021 — Typed Reliquary scopes and Connection state](decisions/0021-typed-reliquary-scopes-and-connections.md)
- [ADR 0020 — Reliquary and Phylactery file kinds](decisions/0020-reliquary-and-phylactery-file-kinds.md)
- [Insomnia semantic validation — 2026-08-24](insomnia-semantic-validation-2026-08-24.md)
- [Roadmap](roadmap.md)
- [Architecture](architecture.md)
- [ADR 0012 — deterministic Episodes and Insomnia Memory authority](decisions/0012-deterministic-episodes-and-insomnia-memory-authority.md)
- [ADR 0017 — CVA workspace and Warlock host application](decisions/0017-cva-workspace-and-warlock-host-application.md)

## Notes

This plan is intentionally provisional where the document marks vocabulary, policy, traversal, migration, or schema details as open. Accepted naming and file-kind decisions remain governed by the referenced ADRs.
