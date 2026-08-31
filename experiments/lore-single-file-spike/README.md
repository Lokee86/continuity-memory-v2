# Lore single-file backend spike

Isolated feasibility experiment for Reliquary roadmap gate 1. This crate does not participate in authoritative REL/PHY storage.

## Pinned Lore upstream

- repository: `https://github.com/EpicGames/lore.git`
- commit: `c49390be048fa273ab47ff7d43bc4994ba7be8ce`
- workspace version at that commit: `0.9.1-nightly`

The crate implements Lore's public `ImmutableStore` and `MutableStore` traits over one append-only file, then runs Lore's own storage and revision machinery against that backend. Cargo restates Lore's vendored `quinn-proto` patch because dependency workspaces do not propagate their root `[patch]` table to standalone consumers.

## Gate result

**Go for roadmap gate 1.** The spike proves the required Lore storage/history substrate over one physical repository artifact without a Lore sidecar/extraction repository directory.

The important merge constraint is that Lore's high-level `merge_start` currently requires an ordinary working-tree path for filesystem verification and realization. The merge proof therefore uses a disposable conventional project folder while all repository storage, branch metadata, immutable fragments, mutable pointers, revisions, and history remain in the single artifact. No `.urc` directory is created. This matches the roadmap target model of an ordinary editable working folder plus one authoritative `.rel`; it does not prove a pathless high-level merge API.

## Verified behavior

`cargo test` currently passes 6/6 tests covering:

- Lore immutable-store `put`/`get`/`query` behavior through `write_content`/`read`;
- Lore mutable-store `load`/`store`/compare-and-swap with reopen persistence;
- Lore fragmentation/chunking on binary content;
- fragment-level compression, observed through Lore compression flags stored by the backend;
- exact-content deduplication across contexts;
- partial chunk reuse after a small same-length binary edit;
- repository/branch creation through a `RepositoryContext` backed only by the artifact;
- multiple commits and historical state checkout by revision hash;
- branch tips, ancestry, divergence, common-ancestor resolution, and `diff3`;
- a real non-conflicting Lore branch merge producing a two-parent merge revision;
- reopen after the merge with branch heads and merged ancestry intact;
- interrupted-tail detection and truncation on reopen;
- eight concurrent readers while one serialized logical writer updates mutable state;
- byte-copy of the artifact to another location followed by successful reopen/history access; and
- absence of a Lore repository sidecar directory during the merge proof.

The tested writer boundary is one logical writer per open artifact, serialized by the backend, with concurrent positioned readers. Cross-process simultaneous mutation of the same physical file is intentionally not supported by this spike and would require an explicit file lease/lock policy.

## Release performance sample

`cargo run --release --example gate_benchmark` writes a 32 MiB deterministic binary corpus through Lore, flushes, closes and reopens the single artifact, then performs 128 pseudo-random 1 MiB Lore reads. Three runs on the current Windows desktop produced:

| Measurement | Range | Mean |
| --- | ---: | ---: |
| Sequential Lore write | 47.35–55.52 MiB/s | 52.20 MiB/s |
| Reopen/index scan of 17.06 MiB artifact | 16.209–20.462 ms | 18.564 ms |
| Random Lore read throughput | 1431.93–1916.25 MiB/s | 1673.47 MiB/s |
| Mean 1 MiB random-read latency | 0.522–0.698 ms | 0.606 ms |

The random-read numbers are warm desktop/OS-cache measurements after reopen, not a cold-device benchmark. They are sufficient for this feasibility gate; scaling and production compaction/indexing remain later measurement work.

The benchmark corpus is deliberately compression-resistant and reported zero compressed payloads. Compression is proven separately by the patterned binary test, which asserts Lore compression flags directly.

## Prototype limitations

This is deliberately not a production storage engine. In particular:

- the physical format is a spike-only checksummed append log;
- the in-memory index is rebuilt by a linear scan on open;
- compaction/eviction are not implemented;
- interrupted recovery only truncates an incomplete/corrupt trailing record;
- no cross-process writer lock is implemented; and
- Lore's high-level merge API still expects a conventional working tree.

Those limitations do not block gate 1.

## Ordinary working-tree proof

Roadmap milestone 3 is also proven in isolation against the same backend. A test creates a conventional project directory containing text, nested files, and a 512 KiB binary, then uses Lore's normal recursive filesystem scan/stage and commit path. It subsequently performs ordinary filesystem operations: text edit, file rename, cross-directory move, file add, explicit delete, and a small binary modification, then scans/stages and commits again.

The proof verifies the first revision directly from Lore history, including the original text and pre-rename/pre-delete paths. It then closes the REL, deletes the entire working directory, reopens only the single REL, recreates an empty target directory, and uses Lore revision sync to materialize the latest committed tree. The reconstructed folder contains the edited/renamed/moved/added binary and text files, preserves the deletion, and creates no `.urc` repository directory.

This establishes the intended operating model for migration work: the working folder is disposable editable materialization; the REL remains authoritative and sufficient to rebuild it.
