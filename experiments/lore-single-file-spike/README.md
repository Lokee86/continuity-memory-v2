# Lore single-file backend spike

Isolated Lore feasibility experiment retained as project-repository evidence. This crate does not participate in authoritative REL/PHY storage. ADR 0027 explicitly keeps Reliquary/Phylactery purpose-built and moves Lore to Warlock's managed project-repository boundary.

## Pinned Lore upstream

- repository: `https://github.com/EpicGames/lore.git`
- commit: `c49390be048fa273ab47ff7d43bc4994ba7be8ce`
- workspace version at that commit: `0.9.1-nightly`

The crate implements Lore's public `ImmutableStore` and `MutableStore` traits over one append-only file, then runs Lore's own storage and revision machinery against that backend. Cargo restates Lore's vendored `quinn-proto` patch because dependency workspaces do not propagate their root `[patch]` table to standalone consumers.

## Feasibility result

**Single-file Lore is technically viable.** The spike proves Lore storage/history over one physical repository artifact without a Lore sidecar/extraction repository directory. That result is retained as evidence and implementation material; it no longer implies that REL/PHY should migrate onto Lore.

The important merge constraint is that Lore's high-level `merge_start` currently requires an ordinary working-tree path for filesystem verification and realization. The merge proof therefore uses a disposable conventional project folder while all repository storage, branch metadata, immutable fragments, mutable pointers, revisions, and history remain in the single artifact. No `.urc` directory is created. This proves that Lore can combine an ordinary editable working folder with a self-contained repository artifact; it does not prove a pathless high-level merge API.

## Verified behavior

`cargo test` currently passes 7/7 tests covering:

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
- byte-copy of the artifact to another location followed by successful reopen/history access;
- absence of a Lore repository sidecar directory during the merge proof; and
- deterministic recovery of two separately diverged single-file Lore artifacts by importing missing foreign immutable Lore objects, resolving their known common ancestor, and persisting a two-parent Lore merge revision.

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

Those limitations do not invalidate the feasibility result.

## Ordinary working-tree proof

The spike also proves the ordinary working-tree model against the same backend. A test creates a conventional project directory containing text, nested files, and a 512 KiB binary, then uses Lore's normal recursive filesystem scan/stage and commit path. It subsequently performs ordinary filesystem operations: text edit, file rename, cross-directory move, file add, explicit delete, and a small binary modification, then scans/stages and commits again.

The proof verifies the first revision directly from Lore history, including the original text and pre-rename/pre-delete paths. It then closes the single repository artifact, deletes the entire working directory, reopens only that artifact, recreates an empty target directory, and uses Lore revision sync to materialize the latest committed tree. The reconstructed folder contains the edited/renamed/moved/added binary and text files, preserves the deletion, and creates no `.urc` repository directory.

This establishes that a normal working folder can be disposable editable materialization while a Lore repository artifact remains sufficient to rebuild it. ADR 0027 no longer requires that artifact to be embedded in the REL.

## OneDrive transport benchmark

The former REL-substrate investigation also produced a local Windows/OneDrive transport sample against the same single-file Lore backend. This is retained as provider-behaviour evidence for any future single-artifact Lore transport option, not as a current REL/PHY migration gate. The reusable driver is `examples/cloud_transport_fixture.rs`.

Environment:

- Windows 11 Home `10.0.26200` (build 26200);
- OneDrive `26.150.0804.0011`;
- primary transport: Wi-Fi, 865 Mbps link;
- artifact: one `.rel`-named experimental file containing Lore repository state and history; this reflects the former substrate experiment, not ADR 0027's production ownership boundary.

Observed results:

| Case | Artifact / logical change | Observed transport/result |
| --- | ---: | --- |
| Initial upload | 67,253,421-byte REL | OneDrive diagnostics recorded exactly 67,253,421 successful upload bytes; `IN_SYNC` after 10.79 s |
| Small incremental commit | +69,502 physical bytes | `IN_SYNC` after 2.63 s; 304,173 adapter bytes sent during operation + sync window |
| Five spaced small commits | +4,287 to +70,189 physical bytes each | all returned to `IN_SYNC` within 5 s; 253,537 to 376,016 adapter bytes sent per window |
| Three-commit burst | +145,738 physical bytes total | `IN_SYNC` after a 30 s quiet window; 657,541 adapter bytes sent |
| 1 MiB binary revision | +1,349,245 physical bytes | `IN_SYNC` after a 20 s quiet window; 1,757,885 adapter bytes sent |
| 16 MiB binary revision | +16,123,472 physical bytes | `IN_SYNC` after a 40 s quiet window; 16,768,758 adapter bytes sent |
| Files On-Demand hydration | 86,589,013-byte online-only REL | local copy completed in 2.95 s; 88,279,953 adapter bytes received; SHA-256 matched exactly |
| Offline/reconnect | +1,302,524 physical bytes while OneDrive was stopped | file remained unsynced offline, then returned to `IN_SYNC` after OneDrive restart |
| Simulated two-REL divergence | two copies committed independently from one Lore ancestor | imported only the foreign immutable objects, resolved the common ancestor with Lore `diff3`, and persisted a two-parent Lore merge revision containing both sides |

These measurements are consistent with OneDrive differential sync rather than whole-artifact retransmission for normal incremental REL writes. Microsoft documents differential sync for all file types and states that only changed portions of large files are transferred during ordinary sync: <https://learn.microsoft.com/en-us/sharepoint/network-utilization-planning>.

One intermittent provider failure was observed. An existing REL remained out of sync after incremental writes even though unrelated OneDrive files synchronized normally. A byte-identical copy under a new filename synchronized successfully, and restarting the OneDrive client later recovered the original file without REL repair. Subsequent isolated 1 MiB, 16 MiB, repeated-small, burst, and offline/reconnect tests did not reproduce the stall. This is provider reliability evidence to retain, not evidence of Lore corruption.

The spike also proves the deterministic recovery half of the conflict requirement without pretending to simulate provider behavior. It copies one REL at a known Lore revision, commits independently in both copies, imports only missing foreign immutable Lore objects into one artifact, publishes the foreign head under a local conflict branch, resolves the common ancestor with Lore `diff3`, and performs a normal Lore merge. Reopen verifies the resulting two-parent revision. Mutable branch pointers are deliberately not bulk-copied between artifacts; head publication remains an explicit conflict-resolution decision.

A genuine two-client simultaneous-write conflict has not been reproduced because only one OneDrive sync client is available in the current environment. Microsoft documents that non-Office conflicts can preserve both versions, with a device-qualified local filename. If Warlock later relies on cloud-synced single-artifact Lore repositories, this provider behavior should still be tested empirically against the already-proven Lore-history recovery path: <https://learn.microsoft.com/en-us/troubleshoot/sharepoint/sync/troubleshoot-sync-issues>.

Dropbox and Google Drive transport behaviour also remains unmeasured in this environment. The OneDrive evidence is sufficient to reject the assumption that a large append-oriented single-file Lore artifact necessarily incurs full-file upload cost, but it does not establish general provider conflict semantics.
