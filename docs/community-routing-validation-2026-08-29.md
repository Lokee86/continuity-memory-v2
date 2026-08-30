# Community routing validation — 2026-08-29

Parent index: [Documentation index](INDEX.md)

## Purpose

Validate whether deterministic owner-local communities can actually reduce Memory-vector retrieval work on a materially larger real Dream Graph without losing useful context or collapsing retrieval into repetitive local results.

This is the evidence gate requested by the earlier [Community routing benchmark — 2026-08-28](community-routing-benchmark-2026-08-28.md).

## Plain-language result

**The approach works on this corpus.**

Instead of comparing a query with every Memory vector, Reliquary can first compare the query with a small derived routing profile for each community, select the four most likely communities, and then perform exact Memory-vector search only inside those communities plus the residual unclustered lane.

On the current 1,087-Memory Project REL this uses about **40.9% of the global vector-comparison work** while preserving **95.2% of the exact global top-four Memory seeds**. With community-aware graph traversal it slightly improves useful-context recall at 16- and 32-node budgets and is effectively tied with the global baseline at 64 nodes.

The locality gain is not coming from filling context with near-duplicate Memories. The selected context stays in fewer graph communities, but mean semantic redundancy rises only about 0.006–0.009 cosine while mean similarity to the query rises slightly.

The evidence supports **four deterministic sub-centroids per community and top-4 community routing** as the normal production candidate. Top-3 remains a plausible explicitly cost-biased mode. Top-2 is too unstable for a general default.

Production retrieval has **not** been changed by this benchmark.

## Fixture

The benchmark uses the real two-week ChatGPT import Project REL:

```text
fixtures/local/chatgpt-first14d/project.prj.rel
```

Current relevant state:

```text
1,087 Memory vectors
1,084 graph nodes
12 Leiden communities
3 vectorized Project Memories outside the Graph/community partition
```

Unclustered vectorized Memories are always admitted as a residual lane so community routing cannot make graph-isolated Memories unreachable.

## Routing representation

A single centroid was previously found to compress multimodal communities too aggressively. Held-out comparison of deterministic sub-centroid counts established four as the useful representation on this corpus: two lost recall, while eight added work without improving recall over four.

The routing representation is therefore:

```text
Community routing profile
→ 4 deterministic sub-centroids
→ derived entirely from member Memory vectors
→ rebuildable index state
```

These vectors are **routing indexes**, not semantic Community objects and not Memory-Web authority. Dream relationships remain authoritative; Leiden communities remain derived structural organization.

## End-to-end retrieval path under test

```text
query
→ score community sub-centroids
→ select top-K communities
→ admit selected community members + unclustered residual Memories
→ exact Memory-vector search inside admitted set
→ take top 4 Memory seeds
→ ordinary graph traversal
   with community preference only within equal traversal depth
```

The baseline is:

```text
query
→ exact global Memory-vector scan
→ top 4 Memory seeds
→ ordinary graph traversal
```

Communities are therefore routing and traversal scheduling hints. They are not semantic supernodes and do not replace Memory-level exact search.

## Validation protocol

The 1,087 vectors are split deterministically into five folds using the first byte of `MemoryId`. For each fold:

- roughly 20% of Memories are held out as queries;
- community routing profiles are built only from the other roughly 80%;
- graph-connected held-out queries are evaluated;
- the five folds cover 1,084 evaluated queries in aggregate;
- exact global top-12 vector neighbours define routing targets;
- exact global top-4 vector neighbours define the seed-preservation baseline;
- graph neighbours of the query define traversal support targets;
- traversal budgets are 16, 32, and 64 nodes.

This prevents the router from constructing its sub-centroids from the query Memory being evaluated.

## K operating-point validation

| K communities | Route top-12 recall | Global top-4 seeds preserved | Total vector work | Δ support @16 | Δ support @32 | Δ support @64 |
| ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 2 | 83.12% | 88.01% | **23.17%** | +0.51 pp | -0.14 pp | **-1.55 pp** |
| 3 | 88.98% | 92.46% | **32.15%** | +0.99 pp | +0.59 pp | -0.55 pp |
| **4** | **92.31%** | **95.16%** | **40.86%** | **+1.18 pp** | **+0.91 pp** | **-0.18 pp** |

`total vector work` counts sub-centroid routing scores plus exact fine-search candidate scores relative to an exact global Memory-vector scan.

### Interpretation

K=2's earlier promising result was split-specific. Across all five folds it becomes meaningfully lossy at deeper traversal and is not robust enough for a general policy.

K=3 is a legitimate lower-cost operating point, but its 64-node tail loss is larger.

K=4 is the stable frontier: it removes roughly 59% of vector comparisons, preserves about 95% of the exact global top-four seeds, improves support retrieval at the two tighter context budgets, and gives up only 0.18 percentage points at 64 nodes.

The evidence does not currently justify traversal-budget-dependent adaptive K. A simpler default K=4 policy is supported; K=3 can remain an explicit cost-biased option if such a mode is later needed.

## K=4 traversal result

| Budget | Global support recall | K=4 support recall | Global communities visited | K=4 communities visited | Global boundary crossings | K=4 crossings |
| ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 16 | 52.23% | **53.41%** | 3.20 | **1.57** | 2.54 | **0.06** |
| 32 | 70.56% | **71.47%** | 5.00 | **2.90** | 6.92 | **2.44** |
| 64 | **85.88%** | 85.70% | 7.68 | **6.00** | 17.21 | **11.22** |

Community preference is applied only within the same traversal depth. This keeps graph depth authoritative while avoiding needless early wandering across community boundaries.

## Context diversity / repetition check

A final measurement tested whether the K=4 result merely looked good because traversal stayed inside one semantic pocket and returned repetitive Memories.

Metrics:

- **dominant-community share:** fraction of returned graph context belonging to the most represented community;
- **effective communities:** entropy-derived count of materially represented communities;
- **semantic redundancy:** exact mean pairwise cosine among returned Memory vectors;
- **query similarity:** mean cosine from returned Memory vectors to the query vector;
- **context volume:** approximate words in returned Memory title/content text.

| Budget | Metric | Global | K=4 |
| ---: | --- | ---: | ---: |
| 16 | Dominant-community share | 76.82% | **90.48%** |
| | Effective communities | 2.15 | **1.31** |
| | Semantic redundancy | 0.5498 | 0.5588 |
| | Mean query similarity | 0.5946 | **0.5981** |
| | Approx. words | 337.85 | 338.15 |
| 32 | Dominant-community share | 67.51% | **79.52%** |
| | Effective communities | 2.82 | **1.86** |
| | Semantic redundancy | 0.5072 | 0.5145 |
| | Mean query similarity | 0.5518 | **0.5538** |
| | Approx. words | 682.87 | 681.60 |
| 64 | Dominant-community share | 56.16% | **70.96%** |
| | Effective communities | 3.89 | **2.69** |
| | Semantic redundancy | 0.4692 | 0.4756 |
| | Mean query similarity | 0.5064 | **0.5081** |
| | Approx. words | 1,377.51 | 1,383.72 |

The graph context becomes intentionally more local, but semantic redundancy rises only:

```text
budget 16: +0.0090 cosine
budget 32: +0.0073 cosine
budget 64: +0.0064 cosine
```

Mean query similarity rises at every budget and context volume is effectively unchanged. The result is therefore **greater graph locality without meaningful semantic collapse**.

The redundancy calculation is exact but linear in context-vector count: vectors are unit-normalized, summed, and the squared norm of that sum is used to recover the mean of all pairwise cosine similarities.

## Timing note

The validation benchmark also records routing, fine-search, global-search, and traversal wall time. The frozen run was a debug-profile test and is useful for instrumentation sanity, not as a production latency claim. The deterministic comparison-count result (`40.86%` vector work for K=4) is the current performance evidence to carry forward. Release-mode latency should be measured as part of production retrieval implementation.

## Decision from this evidence

The large real-corpus evidence gate has passed.

The production candidate is:

```text
4 deterministic sub-centroids / community
→ route top 4 communities
→ exact fine Memory-vector search
→ top 4 Memory seeds
→ graph traversal by depth
   with community preference within depth
```

Constraints remain:

1. Community routing profiles are deterministic derived indexes, never semantic authority.
2. Graph-isolated vectorized Memories remain in an always-admitted residual lane.
3. Dream relationships remain authoritative.
4. Communities do not become semantic supernodes.
5. Production implementation must preserve exact Memory-level fine search inside selected communities.
6. Release-mode latency and production integration tests should be measured when the primitive is implemented.

## Benchmark implementation

The five-fold end-to-end validation and context-quality measurements live in:

```text
src/community_end_to_end_grid_bench.rs
```

Supporting benchmark-only routing/traversal machinery remains under `src/community_*bench*.rs` and `src/community_subcentroid_routing.rs`. No production retrieval behavior is changed by these files.

## Related docs

- [Community routing benchmark — 2026-08-28](community-routing-benchmark-2026-08-28.md)
- [Community scan-and-merge benchmark — 2026-08-27](community-scan-merge-benchmark-2026-08-27.md)
- [ADR 0025 — owner-local derived communities](decisions/0025-owner-local-derived-communities.md)
- [Roadmap](roadmap.md)
