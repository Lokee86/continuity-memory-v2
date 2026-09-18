# ADR 0026: Community-routed owner-local Memory retrieval

Parent index: [Architectural decisions](INDEX.md)

## Status

Accepted and implemented — 2026-08-29.

## Context

Dream already owns semantic Memory-to-Memory relationship authority, and CommunityStore derives deterministic owner-local structural regions from that Graph. A Memory query could exact-score every stored Memory vector, but doing so discards the organization already present in the Memory Web and scales linearly with the complete owner population.

The routing benchmark program established a useful deterministic operating point on a real 1,087-vector Project REL. Four sub-centroids per Community and routing to the top four Communities preserved 95.16% of the exact global top-four Memory seeds while performing 40.86% of the global vector-comparison work. End-to-end Graph support was slightly better at 16- and 32-node budgets and within 0.18 percentage points of global at 64 nodes. Separate context-quality measurement found substantially less Graph wandering without material semantic-redundancy growth.

The routing mechanism must remain derived retrieval machinery. It must not create semantic Community summaries, synthetic Memories, Graph supernodes, or a second relationship authority.

## Decision

Reliquary implements owner-local **Community-routed Memory retrieval** for both REL and PHY.

- `MemoryRetrievalIndex` is transient derived state. It is built from one Compatibility Profile's current non-archived Memory vectors plus the latest Community snapshot when that snapshot matches the current **Memory Graph projection** and Community algorithm.
- The default routing profile derives four deterministic sub-centroids for each Community. Sub-centroids are routing vectors only; they are not persisted and are not semantic objects.
- The default query routes to the top four Communities by the best matching sub-centroid for each Community.
- Vectorized Memories with no Community membership are always admitted as a residual lane. Community routing therefore cannot make a Graph-isolated Memory unreachable.
- Exact cosine fine search runs only over the selected Community members plus that residual lane and returns four Memory seeds by default.
- Graph traversal is undirected over active same-owner relationships for retrieval scheduling. Graph depth remains the primary order. Among candidates at equal depth, paths with fewer Community-boundary crossings are preferred; deterministic discovery order breaks remaining ties.
- Community locality never outranks Graph depth. The rejected boundary-first traversal policy is not part of production retrieval.
- `MemoryRetrievalMode::GlobalExact` remains an explicit whole-owner Memory-vector reference path. A Community-routed query automatically uses the same global path when no current routing profile exists.
- A reusable index records its Compatibility Profile, Memory version, `memory_graph_version`, Community generation, profile-local Memory-vector binding count, vector dimensions, and sub-centroid configuration. Querying a stale cached index fails with `MemoryRetrievalError::StaleIndex`; callers rebuild rather than silently using obsolete routing state. Entity/Observation-only semantic Graph mutations do not stale this Memory-only index. The binding count is a derived in-memory `MemoryVectorStore` index rebuilt on reopen, so newly added vectors invalidate cached retrieval without adding a semantic clock.
- The one-shot `retrieve_memories` convenience method builds an index and executes one query. High-query-rate runtime/Ego code should cache `MemoryRetrievalIndex` and use `retrieve_memories_with_index` until staleness requires rebuilding.
- Query embedding remains outside this primitive. The supplied query vector must already belong to the index's Compatibility Profile space.
- REL and PHY retrieval are strictly owner-local. Cross-owner composition is a higher-level Ego/retrieval responsibility and does not create REL↔PHY Dream candidates or Graph edges.

The benchmark compatibility wrapper delegates to the production sub-centroid implementation so the validated routing algorithm and the shipped algorithm have one source of truth.

## Consequences

The normal Memory retrieval path can avoid scoring most owner-local Memory vectors once the Memory Web contains multiple useful Communities, while retaining an explicit global reference path for comparison and rollback.

Derived routing state can be cached without adding persistence, semantic clocks, reconciliation rules, or Community semantic authority. Memory, Memory-Graph projection, Community, or selected-profile Memory-vector population changes invalidate the cache deterministically through owner watermarks plus a derived profile-local binding count.

The convenience API may rebuild routing state on every call and is therefore not the intended high-throughput integration path. Ego/runtime should own cache lifetime because it already owns repeated context queries and cross-owner composition.

Archived Memories are excluded from both vector seeding and Graph traversal. Non-vectorized non-archived Memories may still be reached through Graph traversal from vectorized seeds.

## Rejected alternatives

### Persist Community vectors as semantic state

Rejected. Sub-centroids are rebuildable retrieval indexes derived from current Memory vectors and Community membership. Persisting them as semantic state would create unnecessary synchronization/versioning obligations and risk treating them as Community meaning.

### Generate LLM Community summaries

Rejected for routing. The routing decision is deterministic and vector/Graph-derived; an inference call would add cost and probabilistic authority without solving a semantic task that ordinary software cannot perform.

### Traverse Community boundaries before Graph depth

Rejected by measurement. A Community-first frontier becomes too sticky and can prefer a deeper local path over a shallower relevant neighbour. Production ordering is `depth -> boundary crossings -> deterministic sequence`.

### Use K=2 as the default

Rejected by five-fold validation. K=2 reduced vector work further but produced unstable recall and a 1.55 percentage-point aggregate support loss at the 64-node budget. K=4 is the validated normal operating point; K=3 remains only a possible explicit cost-biased mode.

### Silently use stale routing indexes

Rejected. Community membership and routing vectors are derived from exact Memory/Memory-Graph-projection state. Reusing an index after those watermarks change can route against obsolete organization. The API fails closed with `StaleIndex` instead.

## Verification

`src/memory_retrieval_tests.rs` protects REL fallback and routed behaviour, the unclustered residual lane, explicit global mode, PHY owner-local routing, stale-index rejection, and equal-depth-only Community traversal preference.

The ignored five-fold end-to-end benchmark and context-quality benchmark exercise the same production sub-centroid implementation through the benchmark compatibility adapter. Frozen large-corpus results are recorded in [Community routing validation — 2026-08-29](../community-routing-validation-2026-08-29.md).

## Related decisions

- [ADR 0013 — Immutable Memory Vector bindings](0013-immutable-memory-vector-bindings.md)
- [ADR 0024 — Owner-local Dream processing](0024-owner-local-dream-processing.md)
- [ADR 0025 — Owner-local derived Graph communities](0025-owner-local-derived-communities.md)
