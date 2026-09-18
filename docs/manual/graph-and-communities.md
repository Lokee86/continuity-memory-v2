# Graph and Communities

Parent index: [Reliquary operator manual](INDEX.md)

## Purpose

Explain which Graph view to use and why Entity relations must not leak into Dream/Leiden behavior.

## Overview

There is one durable Graph relationship authority and two important derived views.

```text
durable typed Graph relations
        │
        ├── full semantic topology
        │     Memory + Entity + future Observation
        │
        └── Memory-only projection
              Dream + duplicate logic + Leiden + Memory retrieval
```

## Full semantic Graph

Use the semantic surface when building a Knowledge graph or Perception feature:

```text
semantic_graph_relations()
semantic_graph_neighbors(node, direction)
entity_associations_for_memory(memory_id)
memories_for_entity(entity_id)
```

Current full-graph relation families include the Memory relation vocabulary plus `EntityAssociation`.

The runtime Knowledge read surface returns current Memories, Entities, active semantic Graph relations, and the current Community overlay. That is the correct source for a user-facing Knowledge graph.

## Memory-only Graph projection

Use these existing APIs when the task is explicitly Memory topology:

```text
graph_relations()
graph_neighbors(memory_id, direction)
shortest_memory_path(source, target, max_depth)
```

These intentionally exclude `Memory -> Entity` associations.

Dream, duplicate-component logic, Leiden Communities, and Memory retrieval use this projection.

## The two Graph clocks

`graph_version()` is the optimistic concurrency token for **all** semantic Graph transactions.

Use it when publishing either Memory relations or Entity associations.

`memory_graph_version()` is a derived watermark: the latest full Graph version that contained a Memory-to-Memory mutation.

Use it only for derived state whose input is the Memory projection.

This distinction means an Entity-only Graph mutation:

- advances `graph_version`.
- does not advance `memory_graph_version`.
- does not invalidate unchanged Communities.
- does not invalidate unchanged Memory retrieval.
- remains visible in the full semantic Graph.

## Communities

`refresh_communities_leiden()` partitions the current owner-local Memory Graph projection.

Entity and future Observation nodes are not Community members merely because they exist in the semantic Graph.

Community snapshots are derived organization, not relationship authority. A current snapshot is keyed to `memory_graph_version`.

## User-facing graph rule

If you are rendering “the Knowledge graph,” do **not** use `graph_relations()` as your source. That method is deliberately the Dream-compatible Memory projection.

Use the full semantic Knowledge/runtime surface instead.

## Related docs

- [Architecture](../architecture.md)
- [Rust API](../api.md)
- [ADR 0036](../decisions/0036-typed-semantic-graph-endpoints.md)
- [Retrieval](retrieval.md)

## Notes

The full topology and Memory projection are two views over the same accepted Graph authority, not two independently writable graphs.
