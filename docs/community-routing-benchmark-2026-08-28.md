# Community routing benchmark — 2026-08-28

Parent index: [Documentation index](INDEX.md)

## Purpose

This benchmark is a routing-only experiment for owner-local Memory-Web communities. It asks whether a small deterministic routing representation can identify the communities containing Memories that a global semantic search would otherwise consider relevant.

It does **not** change production retrieval, Graph traversal, Dream relationships, owner boundaries, or Community authority.

## Overview

The corrected experiment runs over a real typed Project REL populated by Dream, not a reconstructed audit graph. The legacy 46-Memory tuning CVA is first migrated through the normal migration seam, Dream is run to completion against the migrated REL, current Community algorithm v2 is persisted over the resulting Graph, and representative-vector routing is then measured against both manual relation gold and the existing global Memory-vector baseline.

The real Dream run produced 155 active relationships and seven communities across all 46 Memories. Representative routing achieves high recall when five communities are admitted, but five of seven communities admit roughly 77% of all vectors. The corpus therefore still does not demonstrate enough pruning benefit to justify production tiering.

## Correct fixture preparation

The source fixture is `target/insomnia-tuning-v1-check.cva`:

- 314 source turns;
- 11 branches;
- 46 Memories and 46 existing 1024-dimensional Memory vectors;
- legacy Project CVA identity;
- no pre-existing persisted Dream Graph.

The corrected preparation path is:

```text
legacy CVA
→ migrate_file
→ target/dream-routing-baseline.prj.rel
→ Dream with gpt-5.6-sol / low
→ target/dream-routing-live-sol-20260828/dream-run.prj.rel
→ refresh_communities_leiden
→ routing benchmark
```

Migration produced a non-legacy typed Project REL with a durable generated owner UUID while preserving the 314-turn Archive and 46 Memory vectors.

The successful Dream run used the configured `openai-codex` Dream route with `gpt-5.6-sol`, low reasoning, and pair concurrency 12. It completed:

- 46 attempted Memories;
- 0 model/transport failures;
- 0 Memories remaining in `extracted`;
- 46 Graph nodes;
- 155 active relationships;
- 175 relationship mutations;
- Graph version 106;
- 92 Memory revisions / Memory version 92.

Community refresh over that persisted Graph produced seven communities covering all 46 Graph Memories. The benchmark calls `refresh_communities_leiden()` on the typed REL before measuring so the Community snapshot is ordinary persisted derived state rather than a test-only reconstructed partition.

An earlier attempt using the stale configured `stealth/ox-alpha` route failed all 46 calls with OpenRouter/Nous 404 model-not-found responses and produced zero Graph relationships. That failed run is not used for the measurements below; the local Dream route was corrected to the previously validated Codex/Sol-low path.

## Routing profile seam

The test-only experiment derives a transient profile:

```text
persisted Community snapshot
+ existing Memory vectors
→ CommunityRoutingProfile
   → CommunityId
   → representative existing MemoryRefs
```

Representative profiles reuse existing Memory vectors. They do not create semantic Community nodes, synthetic Memories, Graph edges, labels, summaries, or new persistent objects.

The tested existing-Memory strategies are deterministic:

- `medoid`: one Memory maximizing summed cosine similarity to vectorized community members;
- `diverse-4` / `diverse-8`: medoid seed followed by deterministic farthest-first coverage in vector space;
- `structural-4` / `structural-8`: highest unique in-community structural degree, with MemoryId tie-breaking.

A synthetic averaged centroid remains only as a comparison baseline. For multi-representative profiles, a community score is the maximum query-to-representative cosine. Every query uses leave-one-out profile construction so the exact query Memory cannot route trivially through its own stored vector.

## Ground truth

Two independent evaluation lanes are reported:

1. **Audited relation gold.** `corpus/dream-web-gold-v1.json` supplies manually reviewed positive related-Memory pairs from the same 46-Memory corpus. Each positive pair is evaluated in both directions. Because the benchmark now uses the newly persisted Dream Graph rather than reconstructing Graph edges from this gold, this lane is independent structural validation.
2. **Global semantic baseline.** For every vectorized Memory, the existing global Memory-vector cosine baseline selects the nearest 12 other Memories, matching `DEFAULT_DREAM_CANDIDATE_LIMIT = 12`. A baseline Memory is recalled when its actual Dream-derived community appears in the routed top-K.

The second lane remains the primary routing-quality comparison because it measures how much of current global vector discovery would survive coarse Community selection.

## Metrics

Release build on the local development workstation:

| strategy | reps searched | logical routing index | audited recall@1 | @3 | @5 | semantic recall@1 | @3 | @5 | Memories admitted @1 | @3 | @5 | vectors avoided @5 | routing µs/query |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| centroid | 7 | 28,896 B | **88.64%** | **98.30%** | 99.43% | **38.77%** | **77.54%** | 95.65% | 15.17% | **46.41%** | **76.61%** | **23.39%** | 12.4 |
| medoid | 7 | 448 B | 63.64% | 90.91% | **100.00%** | 33.70% | 75.72% | 95.65% | 15.69% | 46.46% | 77.46% | 22.54% | 15.1 |
| diverse-4 | 28 | 1,792 B | 80.11% | 94.89% | 99.43% | 36.96% | 75.00% | 96.38% | 15.26% | 48.39% | 76.98% | 23.02% | 49.5 |
| diverse-8 | 45 | 2,880 B | 85.80% | 96.02% | 99.43% | 37.68% | 75.54% | **97.10%** | 15.36% | 48.16% | 77.27% | 22.73% | 59.1 |
| structural-4 | 28 | 1,792 B | 81.82% | **98.30%** | **100.00%** | 36.96% | 76.45% | 96.74% | 15.36% | 48.53% | 77.32% | 22.68% | 37.7 |
| structural-8 | 45 | 2,880 B | 85.80% | 96.02% | 99.43% | 37.68% | 75.54% | **97.10%** | 15.36% | 48.16% | 77.27% | 22.73% | 57.1 |

`reps searched` is the number of representative vectors examined by this exact brute-force routing experiment. Existing-Memory routing-index size counts only the logical `CommunityId + MemoryId` mapping because vectors already exist in the Memory vector store. Centroid size includes seven synthetic 1024-dimensional `f32` vectors. ANN/index implementation overhead is intentionally not estimated.

Routing latency measures only routing-score evaluation. Leave-one-out representative selection/profile construction is excluded. At 46 Memories the microsecond values are operationally negligible and are not a scaling result.

## Findings

The corrected real-Graph experiment is stronger evidence than the reconstructed run, but it does not change the production decision.

- Dream organizes the 46-Memory corpus into **seven**, not nine, communities.
- Five routed communities recover 95.65–97.10% of the 12-neighbour semantic baseline, depending on representative strategy.
- `diverse-8` and `structural-8` reach the highest semantic recall@5 at **97.10%**, but selecting those five communities still admits **77.27%** of all Memory vectors.
- The centroid gives the best measured top-3 tradeoff: **77.54% semantic recall** while admitting **46.41%** of vectors. That removes about 53.59% of the fine-search population, but loses more than one fifth of the current semantic baseline.
- `structural-4` is the strongest existing-vector structural compromise: 98.30% audited recall@3 and 96.74% semantic recall@5, but it has essentially the same weak top-5 pruning as the other strategies.
- One medoid remains too weak at low K despite reaching full audited recall by K=5.

The limiting factor is the number and size distribution of communities, not routing latency or representative-index size. With only seven communities, a high-recall top-5 route is nearly a whole-web scan by another name.

## Decision

Keep the routing-profile seam and benchmark test-only. Do **not** wire Community selection into production retrieval or Graph traversal from this corpus.

The next evidence gate remains a substantially larger persisted Dream Memory Web with materially more communities. Re-run the same representative experiment there and compare:

```text
global Memory-vector baseline
vs
community route → selected-community fine search
vs
community route → selected-community fine search + small global escape hatch
```

Only advance if the larger graph preserves high recall while eliminating a materially larger fraction of fine-search vectors/nodes. Do not compensate for weak routing evidence by adding LLM Community summaries, semantic Community supernodes, cross-owner communities, hierarchy, lineage machinery, or changed-region Leiden.

## Related docs

- [Architecture](architecture.md)
- [Current limitations](current-limitations.md)
- [Roadmap](roadmap.md)
- [Community scan-and-merge benchmark — 2026-08-27](community-scan-merge-benchmark-2026-08-27.md)
- [ADR 0025 — owner-local derived communities](decisions/0025-owner-local-derived-communities.md)

## Notes

This is an ignored release benchmark over a local typed REL produced by normal migration and Dream execution. `RELIQUARY_ROUTING_REL` can override the default fixture path for reruns against another persisted Dream REL. The benchmark is an engineering measurement artifact, not a production retrieval contract.