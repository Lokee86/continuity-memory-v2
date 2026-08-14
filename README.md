# Continuity Memory v2

Clean Rust rebuild of Continuity's storage and runtime architecture.

A `.cva` is one physical container for several purpose-built databases. The container owns physical storage and CVA-global ordering; each database owns its semantic records, indexes, local mutation/version semantics, and derived state.

## Current status

Implemented now:

- CVA format header and append-only opaque chunks;
- CVA-global monotonic `u64` version tickets;
- Archive-local contiguous `u64` mutation watermarks;
- dual `(global_version, archive_version)` metadata for every semantic Archive node, branch revision, or fragment;
- Archive content bodies addressed by SHA-256;
- immutable branch-aware Archive nodes with conversation-local parent ancestry;
- append-only branch/session-head revisions with historical lookup;
- revival of an old conversation point through a new local branch without rewinding the Archive;
- durable 8-turn / 2-overlap retrieval fragment ranges;
- reopen-time reconstruction and validation of Archive state;
- compact Archive-owned node/branch/fragment lookup indexes without composite string keys;
- unit tests and a prepared graph-corpus create/close/reopen round-trip smoke test.

The current Archive format is development-only `CVAAFMT2`. Earlier development formats are rejected; migration code is intentionally not implemented yet.

Not implemented yet:

- persistent Archive checkpoints;
- compression, checksums, encryption, packing, reclamation, or concurrent writer coordination;
- a general materialized historical `ArchiveView` API;
- whole-CVA restore-and-continue across multiple databases;
- Memories, Graph, Archive Vector, or Memory Vector databases;
- shared Continuity runtime, retrieval ranking, Insomnia, Dream, or Ego.

The current compact indexes remain derived acceleration state; they do not alter Archive authority or historical semantics. Open still performs redundant physical chunk scans that are the next measured startup target.

## Architecture rule

> Defer mechanics, not ownership.

Storage mechanisms may remain simple while owning database boundaries remain explicit. The project does not use a generalized semantic database/root/dependency layer. Integer version adjacency provides ordering and historical cuts; it does not define semantic ancestry.

## Documentation

Start with [the documentation index](docs/INDEX.md), [current architecture](docs/architecture.md), [storage format](docs/storage-format.md), and [current limitations](docs/current-limitations.md).

Implemented Archive versioning plus the remaining whole-CVA historical-recovery plan are documented in [versioning, historical cuts, and rollback](docs/version-history-plan.md).