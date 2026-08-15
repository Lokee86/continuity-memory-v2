# Continuity Memory v2

Clean Rust rebuild of Continuity's storage and runtime architecture.

A `.cva` is one physical container for several purpose-built databases. The container owns physical storage and CVA-global ordering; each database owns its semantic records, indexes, local mutation/version semantics, and derived state.

## Current status

Implemented now:

- purpose-built `continuity.cfg` current-state configuration with replaceable typed objects and atomic whole-file replacement;
- configurable fragment and retrieval policy with the original defaults preserved;
- expandable model-switchboard routing with General/Embedding capabilities and `openai-codex` / `openai-ready` providers;
- self-generated 256-bit master key with a temporary local JSON key store, ready for later credential encryption;
- CVA format header and append-only opaque chunks;
- CVA-global monotonic `u64` version tickets;
- Archive-local contiguous `u64` mutation watermarks;
- dual `(global_version, archive_version)` metadata for every semantic Archive node, branch revision, or fragment;
- Archive content bodies addressed by SHA-256;
- immutable branch-aware Archive nodes with conversation-local parent ancestry;
- append-only branch/session-head revisions with historical lookup;
- revival of an old conversation point through a new local branch without rewinding the Archive;
- durable 8-turn / 2-overlap retrieval fragment ranges;
- `Cva` as the single physical composition owner for one Container plus concrete stores;
- single-pass reopen reconstruction that feeds Archive, packed-vector, Archive-Vector, compatibility-profile, and vector-generation rebuild from the same physical scan;
- compact Archive-owned node/branch/fragment lookup indexes without composite string keys;
- a Lodestone-derived generic packed-vector representation supporting arbitrary non-zero dimensions and scalar widths from int8 through float64;
- immutable SHA-256-addressed packed-vector matrix objects stored inside the CVA without consuming semantic clocks;
- immutable Archive-Vector sets that bind packed-vector row ordinals to Archive `FragmentId`s with exact row/reference validation;
- immutable endpoint-independent compatibility profiles with Query/Document reference probes, tolerant cosine verification, and deterministic simulated endpoints for development;
- versioned vector-generation publication that binds one profile to one Archive-Vector set and Archive coverage watermark;
- independent current generations per profile plus historical generation lookup by vector-version cut;
- exact semantic retrieval over the selected profile's current generation: endpoint compatibility verification, Query-mode embedding, exact cosine scan, and row-to-fragment resolution;
- restored default hybrid retrieval: 30 lexical + semantic candidates, `0.45/0.55` score fusion when both channels match, range deduplication, overlap diversification, and top-10 results;
- unit tests plus prepared-corpus Archive and two-profile vector-generation/retrieval smoke tests.

The current development format requires Archive `CVAAFMT2`, packed-vector `CVAPVFM1`, Archive-Vector `CVAAVFM1`, compatibility-profile `CVACPFM1`, and vector-generation `CVAVGFM2` markers. Earlier development CVAs are rejected; migration code is intentionally not implemented yet.

Not implemented yet:

- persistent Archive checkpoints;
- compression, checksums, encryption, packing, reclamation, or concurrent writer coordination;
- a general materialized historical `ArchiveView` API;
- whole-CVA restore-and-continue across multiple databases;
- encrypted credential objects, production OS credential-store master-key integration, and direct provider authentication/transport;
- live embedding-provider execution through the switchboard, ANN search, reranking, search filters, or a broader retrieval-controller policy;
- quantization metadata/alternate generation-builder encodings;
- Memories, Graph, or Memory Vector databases;
- shared Continuity runtime, Insomnia, Dream, or Ego.

The current indexes remain derived acceleration state. Archive and Vector Generations are the two mutable semantic timelines; compatibility profiles, packed matrices, and Archive-Vector bindings are immutable backing objects. Reopen performs one streaming physical chunk pass. Current measurements are in `docs/development.md`; a current-format cold-cache sample and larger realistic-dimension vector measurements remain required.

## Architecture rule

> Defer mechanics, not ownership.

Storage mechanisms may remain simple while owning database boundaries remain explicit. The project does not use a generalized semantic database/root/dependency layer. Local configuration is a separate purpose-built replaceable file, not another semantic database. Integer version adjacency provides ordering and historical cuts; it does not define semantic ancestry.

## Documentation

Start with [the documentation index](docs/INDEX.md), [current architecture](docs/architecture.md), [local configuration](docs/configuration.md), [storage format](docs/storage-format.md), and [current limitations](docs/current-limitations.md).

Implemented Archive versioning plus the remaining whole-CVA historical-recovery plan are documented in [versioning, historical cuts, and rollback](docs/version-history-plan.md).
