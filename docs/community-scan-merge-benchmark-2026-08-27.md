# Community scan-and-merge benchmark — 2026-08-27

Parent index: [Documentation index](INDEX.md)

## Purpose

This freezes the first synthetic scaling gate for community algorithm v2. It compares the implemented deterministic graph-local scan-and-merge reduction with one monolithic Leiden invocation and checks the resulting modularity rather than timing an approximation without a quality reference.

## Overview

The benchmark tests scaling, reduction depth, sensitivity to shard layout, and partition quality. It is evidence for the current algorithm choice, not a hardware-independent service-level target.

## Method

The benchmark is the ignored release test `community_scan_merge_bench::synthetic_scan_merge_benchmark` in `src/semantic_graph/community_scan_merge_bench.rs`.

Synthetic Graph characteristics:

- planted communities of 64 nodes;
- four forward in-community neighbors per node, deduplicated as undirected structural edges;
- one sparse bridge per planted community;
- approximately four structural edges per node;
- community scan shard target: 2,048 nodes;
- binary merge fan-in;
- 16 available workers on the benchmark host;
- Leiden modularity resolution `1.0` and seed `0x4c454944454e0001` for both paths.

Two equivalent logical layouts are tested. `local` numbers planted-community nodes contiguously. `interleaved` deliberately interleaves those same logical communities in dense node order. Algorithm v2 first derives deterministic BFS graph-local scan order, so the second layout checks that shard quality is not merely an artifact of insertion order.

Times are release-build wall-clock observations from one development machine, not hardware-independent performance guarantees. The modularity delta is `scan-and-merge quality - monolithic quality`.

## Results

| Layout | Nodes | Edges | Shards | Merge levels | Monolithic ms | Scan/merge ms | Speedup | Monolithic Q | Scan/merge Q | Q delta |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| local | 1,024 | 4,112 | 1 | 0 | 1.562 | 2.425 | 0.64× | 0.93360895 | 0.93360895 | 0.00000000 |
| local | 8,192 | 32,896 | 4 | 2 | 15.889 | 13.523 | 1.18× | 0.98829645 | 0.98557476 | -0.00272169 |
| local | 65,536 | 263,168 | 32 | 5 | 583.264 | 79.430 | 7.34× | 0.99598023 | 0.99583057 | -0.00014966 |
| local | 262,144 | 1,052,672 | 128 | 7 | 19,936.168 | 328.713 | 60.65× | 0.99800527 | 0.99799794 | -0.00000734 |
| interleaved | 1,024 | 4,112 | 1 | 0 | 2.850 | 4.344 | 0.66× | 0.93360895 | 0.93360895 | 0.00000000 |
| interleaved | 8,192 | 32,896 | 4 | 2 | 30.983 | 14.777 | 2.10× | 0.98829645 | 0.98602879 | -0.00226766 |
| interleaved | 65,536 | 263,168 | 32 | 5 | 1,056.669 | 87.624 | 12.06× | 0.99598023 | 0.99582678 | -0.00015345 |
| interleaved | 262,144 | 1,052,672 | 128 | 7 | 28,440.925 | 405.664 | 70.11× | 0.99800458 | 0.99799610 | -0.00000848 |

A separate scan-and-merge-only case used **1,048,576 nodes / 4,210,688 edges / 512 shards / 9 merge levels** and completed in **1,247.455 ms** with reported modularity `0.99900167`. An exact monolithic 1M-node reference is intentionally not reported; the attempted reference dominated the benchmark after the 262K monolithic cases were already taking roughly 20–28 seconds, so no unmeasured 1M speedup is claimed.

## Findings

Scan-and-merge has overhead below the shard threshold and only begins to win once multiple meaningful shards exist. By 65K nodes it is approximately 7–12× faster on these sparse graphs; by 262K it is approximately 61–70× faster.

The binary reduction tree has 7 serial merge levels at 128 shards and 9 at 512 shards. This is the intended logarithmic reduction depth. It does **not** mean total clustering work is `O(log N)`: graph scanning, edge assignment, leaf clustering, and summary construction still process the input population.

Dense insertion-order sharding was rejected during this benchmark because the interleaved layout materially reduced modularity. Switching the scan phase to deterministic graph-local BFS ordering reduced the final interleaved modularity delta to about `-0.00015` at 65K and `-0.0000085` at 262K while preserving the scaling advantage.

The 8K cases show the largest remaining proportional quality difference, roughly `-0.0023` to `-0.0027`. Algorithm v2 is therefore explicitly an approximate hierarchical reduction, not a promise to reproduce the exact monolithic Leiden partition. The stored quality is separately protected by a unit test that recomputes modularity for the final partition over the original structural Graph.

## Advancement decision

The synthetic gate supports keeping scan-and-merge as the complete community organization path. No changed-region incremental machinery is justified by these results. Further incremental invalidation or reduction-tree reuse should be driven by measurements on real large REL/PHY Graphs rather than assumed necessary in advance.

## Related docs

- [Architecture](architecture.md)
- [ADR 0025: Owner-local derived Graph communities](decisions/0025-owner-local-derived-communities.md)
- [Current limitations](current-limitations.md)
- [Roadmap](roadmap.md)

## Notes

The benchmark fixture is intentionally synthetic and sparse. Real REL/PHY Graph density, community-size distribution, and relationship topology should be measured separately before tuning the 2,048-node shard target or binary merge fan-in.
