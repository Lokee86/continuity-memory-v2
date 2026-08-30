# Community routing benchmark — 2026-08-28

Parent index: [Documentation index](INDEX.md)

## Purpose

Measure whether owner-local Dream communities can act as a useful first retrieval tier before fine Memory-vector search, while preserving the quarantine of the obsolete monolithic 46-Memory fixture.

## Overview

The benchmark now has two complementary lanes: a real persisted owner-local Dream graph for routing quality and a deterministic large synthetic layout for scaling economics. The real lane compares representative strategies against an oracle partition ceiling and always admits vectorized Memories without community membership; the synthetic lane measures representative-routing cost and vector-work reduction without claiming real semantic quality.

## Status

**Superseded as the final routing-quality gate by [Community routing validation — 2026-08-29](community-routing-validation-2026-08-29.md).** This document remains the frozen small owner-split and synthetic scale baseline that motivated the larger real-corpus validation.

The old 46-Memory Project-only run remains superseded and quarantined under:

```text
target/quarantine/legacy-46-memory-routing-20260828/
```

The current benchmark requires an explicit `RELIQUARY_ROUTING_REL`; there is intentionally no fallback fixture.

## Current owner-split fixture

The replacement fixture was regenerated from the frozen 11-Episode / 314-turn source corpus through current owner routing and then completed owner-locally by Dream. It is preserved locally outside `target/`:

```text
fixtures/local/owner-split-dream-v1/project.prj.rel
fixtures/local/owner-split-dream-v1/user.phy
```

Current persisted state:

```text
Project REL
- 43 Memories
- 43 Memory vectors
- 129 active Dream relations
- 9 communities
- Dream backlog zero

User PHY
- 9 Memories
- 9 Memory vectors
- 11 active Dream relations
- 4 communities
- Dream backlog zero
```

This is a valid owner-split mechanics fixture, not a controlled Dream-quality benchmark. The source-identical Insomnia rerun produced 43 Project / 9 User Memories rather than the historical 41 / 8 result, demonstrating expected inference variance. The rerun occurred while the local main-Insomnia route was temporarily set to Sol-low rather than the intended Sol-high configuration. Dream completion also used bounded fallback models after provider quota/failure events. REL and PHY remained strictly owner-local throughout.

Project routing is measured against the Project REL only. The nine-Memory PHY is too small to provide useful routing evidence.

## Residual Memory requirement

The Project REL contains 43 vectorized Memories but only 39 Memories participating in the persisted Graph communities. Four Memories are graph-isolated and therefore have no community membership.

A community-routing tier must not make those Memories unreachable. The valid benchmark therefore treats unassigned vectorized Memories as an **always-admitted residual lane**:

```text
query
→ route top-K communities
→ admit selected community members
+ admit all vectorized Memories with no community
→ fine Memory-vector search
```

The earlier benchmark code silently excluded unassigned Memories from its semantic target set. That measurement was invalid for end-to-end retrieval and has been corrected.

## Real persisted-graph benchmark

Release-mode run:

```text
RELIQUARY_ROUTING_REL=fixtures/local/owner-split-dream-v1/project.prj.rel
cargo test --release community_routing_strategy_benchmark -- --ignored --nocapture
```

The semantic lane uses every one of the 43 Memory vectors as a leave-one-out query. For each query, full-vector cosine search defines the top 12 other Memories as the baseline target set, yielding 516 target instances. The four residual Memories remain eligible targets and are always admitted.

The legacy audited-gold IDs overlap this regenerated fixture in only two query/target pairs. Their 100% scores are retained only as a smoke check and are not meaningful quality evidence.

`oracle` chooses the best possible K communities for each query from the known target distribution. It measures the ceiling imposed by the partition itself rather than a realizable router.

| Strategy | Reps | Top-1 recall | Top-3 recall | Top-5 recall | Top-5 admitted | Top-5 avoided | Route µs/query |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| Oracle ceiling | 0 | 49.22% | 83.53% | **97.67%** | 71.65% | 28.35% | — |
| Centroid | 9 | 45.74% | 79.07% | 96.12% | 76.36% | 23.64% | 18.6 |
| Medoid | 9 | 44.38% | 71.32% | 86.24% | 67.33% | 32.67% | 17.8 |
| Diverse-4 | 28 | 41.67% | 78.68% | 96.12% | 72.76% | 27.24% | 48.2 |
| Diverse-8 | 37 | 43.60% | 79.26% | **97.48%** | 75.30% | 24.70% | 64.6 |
| Structural-4 | 28 | 43.41% | 78.10% | 96.12% | 73.03% | 26.97% | 51.4 |
| Structural-8 | 37 | 43.60% | 78.88% | 97.29% | 75.14% | 24.86% | 63.2 |

### Interpretation

Top-5 is the first useful operating point on this fixture. Top-1 and top-3 discard too much of the full-vector semantic neighbourhood.

`diverse-8` reaches 97.48% target recall versus a 97.67% oracle ceiling: only 0.19 percentage points below the best recall the nine-community partition can provide at K=5. Representative selection is therefore not the material limiter at top-5 on this graph; the partition itself accounts for almost all remaining loss.

The 43-Memory graph is too small for meaningful pruning. Even the best high-recall lane still admits 75.30% of the leave-one-out candidate vectors after the mandatory residual lane, avoiding only 24.70%. That is a fixture-size limitation, not evidence that the tier cannot scale.

The centroid lane is efficient and nearly as accurate, but it requires a derived Community vector. Representative lanes deliberately reuse existing Memory vectors and therefore remain the preferred architecture unless larger evidence justifies derived centroid vectors.

## Synthetic scale benchmark

A separate ignored release benchmark measures routing/work scaling without pretending to provide semantic-quality evidence. It uses deterministic, well-separated 1024-dimensional synthetic clusters with 64 Memories per community, 64 out-of-sample queries, no residual Memories, and 1/4/8 existing-Memory representatives per community.

```text
cargo test --release synthetic_community_routing_scale_benchmark -- --ignored --nocapture
```

`total vector work` is the representative scores required for routing plus the Memories admitted by the top five communities, divided by a full Memory-vector scan. `work reduction` is the reciprocal. It is a deterministic comparison-count model; only the routing latency column is measured wall-clock time.

| Memories | Communities | Reps/community | Top-5 fine-search avoided | Total vector work | Work reduction | Route ms/query |
| ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 1,024 | 16 | 1 | 68.75% | 32.81% | 3.05× | 0.023 |
| 1,024 | 16 | 4 | 68.75% | 37.50% | 2.67× | 0.084 |
| 1,024 | 16 | 8 | 68.75% | 43.75% | 2.29× | 0.160 |
| 8,192 | 128 | 1 | 96.09% | 5.47% | 18.29× | 0.200 |
| 8,192 | 128 | 4 | 96.09% | 10.16% | 9.85× | 0.861 |
| 8,192 | 128 | 8 | 96.09% | 16.41% | 6.10× | 1.482 |
| 65,536 | 1,024 | 1 | 99.51% | 2.05% | 48.76× | 1.629 |
| 65,536 | 1,024 | 4 | 99.51% | 6.74% | 14.84× | 6.971 |
| 65,536 | 1,024 | 8 | 99.51% | 12.99% | 7.70× | 11.908 |
| 262,144 | 4,096 | 1 | 99.88% | 1.68% | **59.36×** | 6.374 |
| 262,144 | 4,096 | 4 | 99.88% | 6.37% | **15.69×** | 25.055 |
| 262,144 | 4,096 | 8 | 99.88% | 12.62% | **7.92×** | 47.117 |

Synthetic routing recall is 100% for all rows because the benchmark intentionally uses separable clusters. Those recall values are a harness sanity check only and must not be compared with the real persisted-graph recall above.

The scale result establishes the expected economics of a flat representative tier: with fixed 64-Memory communities, routing over four representatives per community plus fine-searching five communities reduces vector comparison work by roughly 15–16× once the Memory Web reaches tens or hundreds of thousands of Memories. The current brute-force representative scorer remains linear in representative count; at 262,144 Memories the four-representative lane costs about 25 ms/query in release mode before fine search. A representative ANN/index would be a separate optimization and is not assumed by this result.

## Decision

Communities now have demonstrated retrieval value, but the evidence is not yet sufficient to promote routing into the production retrieval path.

What is established:

1. Owner-local communities can serve as a first-tier retrieval partition without Community semantic summaries or LLM-generated Community objects.
2. Graph-isolated vectorized Memories require an always-searched residual lane.
3. On the real fixture, eight diverse existing-Memory representatives reach essentially the top-5 recall ceiling of the partition.
4. On deterministic large synthetic layouts, representative routing materially reduces total vector comparison work as the number of communities grows.

What is not yet established:

1. That real large Memory Webs retain similarly bounded community sizes and semantic locality.
2. That approximately 97.5% target-level recall is sufficient for production retrieval, or whether later reranking/multi-tier traversal recovers the remaining tail.
3. That eight representatives per community is the right long-term operating point; the real fixture is too small to resolve the quality/work tradeoff confidently.

Keep Community routing test-only until a materially larger persisted owner-local Dream corpus validates recall and pruning together. The next useful evidence is a large real Memory Web, not additional tuning against this 43-Memory fixture.

## Related docs

- [ADR 0024 — owner-local Dream processing](decisions/0024-owner-local-dream-processing.md)
- [ADR 0025 — owner-local derived communities](decisions/0025-owner-local-derived-communities.md)
- [Community scan-and-merge benchmark — 2026-08-27](community-scan-merge-benchmark-2026-08-27.md)
- [Development](development.md)

## Notes

The local REL/PHY fixture pair is intentionally excluded from Git. Benchmark code is tracked; generated owner data is not. The synthetic scale benchmark measures the current brute-force test router rather than an ANN implementation.
