# Architecture

## Mental model

A Continuity Vault Archive is one portable transactional container containing multiple explicit databases:

```text
CVA container
├── Archive DB
├── Memories DB
├── Graph DB
├── Archive Vector DB
└── Memory Vector DB
```

The databases are cross-indexed through stable IDs, but each database owns its own records, indexes, mutation rules, and current-state representation.

Examples:

```text
Memory.evidence_fragment_id  -> Archive.Fragment.id
GraphEdge.source_memory_id   -> Memories.Memory.id
GraphEdge.target_memory_id   -> Memories.Memory.id
ArchiveVector.fragment_id    -> Archive.Fragment.id
MemoryVector.memory_id       -> Memories.Memory.id
MemoryVector.revision        -> Memories.Revision
```

## Container responsibility

The CVA container owns only common physical guarantees:

- append/store immutable chunks;
- stable addressing;
- integrity/authentication;
- coherent snapshots;
- atomic publication of one or more changed database states;
- crash recovery;
- reachability and reclamation/vacuum;
- immutable snapshot reads/pins.

The container must not interpret domain semantics.

It must not know:

- whether an Archive change is authoritative or derived;
- whether a vector depends on a fragment;
- whether a graph edge depends on a memory revision;
- whether two domain states are "semantically equivalent";
- how a database determines stale work or rebuildability.

Those rules belong to the database that owns the data.

## Database responsibility

Each mutable domain is implemented as a separate database.

Each database explicitly owns:

- durable record schema;
- stable IDs;
- indexes;
- authoritative versus derived state within that database;
- mutation validation;
- local concurrency/locking;
- current-state pointer/manifest if needed;
- compaction/checkpoint policy;
- external references to other databases;
- validation of those references where required.

Duplication of small amounts of implementation is preferred over creating a generalized semantic database framework.

## Cross-database references

Databases reference one another through stable logical IDs, not shared mutable objects.

A cross-database reference is owned by the database containing the reference. The common container does not register or resolve a generic dependency graph.

If a Graph record references Memory `M42`, Graph owns that reference and Graph code decides when/how to validate it.

If an Archive vector maps to Fragment `F17`, the Archive Vector database owns that mapping and its source-version rules.

## Atomic multi-database publication

Some product operations legitimately change multiple databases at once.

The flow is explicit:

```text
prepare Archive DB state (if changed)
prepare Memories DB state (if changed)
prepare Graph DB state (if changed)
prepare vector DB state (if changed)
        ↓
container atomically publishes the selected new database states
```

The container provides all-or-nothing visibility. It does not understand why the states belong in one transaction.

## Operational state

Process/runtime work state is not a semantic database inside the CVA.

Queues, leases, retries, worker cursors, and transient scheduling state may live in disposable SQLite or equivalent runtime storage when needed.

Deleting runtime state must not make the CVA semantically ambiguous or invalidate durable memory.

## Reuse from the previous repository

Nothing is copied merely because it already exists.

Low-level primitives may be reused only after they are shown to fit this architecture without importing semantic generalization.

Likely reusable categories include:

- immutable physical sections/packs;
- addressing and integrity primitives;
- atomic file publication/recovery;
- snapshot pinning;
- encryption/container identity;
- reachability/vacuum primitives;
- domain record codecs and validation that remain domain-local.

Generalized semantic publication/dependency machinery is not a target architecture.
