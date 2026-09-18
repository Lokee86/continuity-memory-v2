# Core Concepts

Parent index: [Reliquary operator manual](INDEX.md)

## Purpose

Provide the minimum vocabulary needed to use Reliquary correctly.

## Overview

Reliquary deliberately separates semantic owners instead of collapsing everything into one generic database.

### REL

A Reliquary is an owner-local semantic container for a project, organization, or connection scope. It can contain Archive history, Memories, Entities, Graph state, derived Communities, vectors, Ego state where applicable, and runtime-owned records.

### PHY

A Phylactery is the user-global Memory owner. It can be attached to an active REL workflow, but it does not copy REL transcript/Episode payloads. Routed User Memories retain identifier-only source references back to the originating REL.

### Archive

Archive is source history: turns, branches, conversation metadata, attachments, Fragments, Episodes, and related provenance.

Archive answers questions such as:

- What did the source conversation actually contain?
- Which exact turn or Episode grounded this Memory?
- What did an older branch say?

### Memory

A Memory is a source-grounded proposition extracted from user-authoritative source material. Memory identity is stable across revisions. Memory bodies/revisions are durable semantic authority.

### Entity

An Entity is a durable canonical referent mentioned by Memories: a person, project, service, file-like referent, organization, and so on.

Entity is a sibling semantic object, not a subtype of Memory.

Current Entity storage supports stable IDs, metadata revisions, canonical names, aliases, persistence/reopen, migration/reconciliation, and Graph association.

### Observation

An Observation is intended to be a higher-order proposition inferred from multiple pieces of knowledge. The semantic node kind exists, but there is not yet a durable Observation owner or accepted production relation family. Treat Observation workflows as planned.

### Graph

Graph owns semantic relationships between owner-local semantic nodes.

Current relation families include:

- Dream/User Memory-to-Memory relations.
- Perception-owned `Memory -> Entity` `EntityAssociation`.

The full semantic Graph is distinct from the Memory-only projection used by Dream and Leiden.

### Episode

An Episode is a deterministic source window used as semantic processing authority. Insomnia operates over Episodes; PHY does not own Episodes.

### Insomnia

Insomnia extracts durable Memories from source Episodes. It also performs metadata/routing enrichment, including Entity mention spans. Extraction is not Entity identity resolution.

### Dream

Dream reasons over Memory relationships and publishes verified Memory-to-Memory Graph structure, lifecycle state, duplicate/supersession behavior, and related derived organization.

### Perception

Perception owns Entities and future Observations. Durable Entity storage, Memory-to-Entity Graph associations, bounded candidate retrieval, zero-candidate Admission, calibrated V4 identity resolution, and deterministic per-mention processor persistence are implemented. The organic zero-Entity bootstrap is corpus-proven; automatic runtime scheduling of that processor is not yet wired.

### Durable vs derived

Examples of durable semantic authority:

- Memory revisions.
- Entity revisions.
- accepted Graph relationship transactions.

Examples of derived/replaceable state:

- lexical indexes.
- Memory retrieval indexes.
- Arcana topology projections.
- Community snapshots/names are persisted derived organization, not relationship authority.

## Related docs

- [Architecture](../architecture.md)
- [Architectural invariants](../invariants.md)
- [Processing pipeline](processing-pipeline.md)
- [Graph and Communities](graph-and-communities.md)

## Notes

When two pieces of state look similar, check which owner is authoritative before writing or caching them.
