# Architecture

Parent index: [Documentation index](INDEX.md)

## Purpose

This document owns the current Continuity Memory v2 implementation boundaries, state ownership, lifecycle, and code map.

## Overview

A `.cva` is one physical file containing several explicit owners:

```text
Cva
├── Container
│   └── global_version: u64
├── Archive
│   ├── archive_version: u64
│   └── conversation/session-local ancestry
├── PackedVectorStore
│   └── immutable numeric matrices
├── ArchiveVectorStore
│   └── immutable row -> FragmentId bindings
├── EmbeddingProfileStore
│   └── immutable vector-space identities
└── VectorGenerationStore
    └── vector_version: u64 + current generation per profile
```

`Cva` owns physical composition and one Container handle. It does not merge these owners into a generalized semantic database.

## Responsibilities

### Container

Container owns the fixed header, opaque length-prefixed chunks, `ChunkRef`, file I/O, sync, one physical reopen scan, and CVA-global monotonic version tickets. Global version is ordering only.

### Archive

Archive owns source-history semantics: content-addressed text, immutable conversation nodes, conversation-local parent ancestry, branch/session-head revisions, fragment ranges, the dense Archive watermark, historical branch lookup, and Archive-owned derived indexes.

`archive_version` is a whole-Archive mutation cut. It is not conversation ancestry.

### PackedVectorStore

Packed vectors own immutable matrix bytes and physical row representation. A `VectorSchema` defines dimensions and scalar representation; rows are fixed-width and contiguous. Equal schema+bytes deduplicate.

Packed vectors do not know which Archive records rows represent or which embedding space produced them.

### ArchiveVectorStore

Archive Vectors own one relationship only:

```text
ArchiveVectorSet
├── packed_vector_id
└── ordered FragmentIds
    row N -> fragment_ids[N]
```

Creation/reopen require an existing matrix, exact row count, real unique Archive fragments, and valid content identity. The persistent object contains no profile/model/metric/source watermark.

The derived Archive-Vector index also records the newest Archive creation version among mapped fragments. That value is not persistent identity; it exists to validate generation coverage.

### EmbeddingProfileStore

An embedding profile is immutable vector-space identity:

```text
provider / model / revision
dimensions
normalization
probe-suite version
behavior fingerprint
```

Profile identity deliberately excludes packed scalar representation, Archive mappings, and generation state. Profile creation is clock-neutral.

The endpoint abstraction has separate query/document modes. Current tests/building use deterministic simulated endpoints only. Probe behavior is fingerprinted so two endpoints advertising the same metadata but producing different vectors receive different profile IDs.

### VectorGenerationStore

Vector Generations own active vector-population publication:

```text
VectorGeneration
├── profile_id
├── archive_vector_id
├── source_archive_version
├── global_version
└── vector_version
```

`vector_version` is a dense local watermark for generation publications. The newest generation for each profile is the derived current generation; older generations remain retained and can be resolved at a historical vector-version cut.

Generation publication is the first vector-layer operation that consumes a CVA-global ticket. Source Archive versions cannot regress for a profile and cannot predate any mapped fragment.

## Write lifecycles

### Archive semantic mutation

```text
append Archive payload
    ↓
allocate global version G
    ↓
allocate Archive version A
    ↓
append ArchiveRecordVersion { G, A, record }
```

An unversioned node/branch/fragment payload is inert.

### Vector generation publication

```text
profile + packed matrix + ArchiveVectorSet already exist
    ↓
append immutable generation payload
    ↓
allocate global version G
    ↓
allocate vector version V
    ↓
append generation metadata { G, V, record }
```

An unversioned generation payload is inert. Failed builders may leave unreferenced backing objects, but those objects consume no semantic versions.

### Development generation builder

`build_archive_vector_generation` verifies a supplied endpoint against the stored profile, takes the current Archive watermark, embeds all durable fragments in deterministic `FragmentId` order using document mode, writes an `f32` packed matrix and Archive-Vector binding, then publishes the generation.

## Reopen

```text
one physical chunk scan
    ├── Container framing/global-ticket validation
    └── Cva dispatches each payload to concrete rebuild states
        ├── Archive
        ├── PackedVectorStore
        ├── ArchiveVectorStore
        ├── EmbeddingProfileStore
        └── VectorGenerationStore
```

After the scan, cross-store references are validated in dependency order. Full packed matrices and Archive-Vector mappings are not retained in steady-state indexes.

## Ordering model

Archive and Vector Generations are now two independently mutable semantic domains. Their local clocks can interleave through the shared global clock:

```text
G100 / A700   Archive mutation
G101 / V20    vector generation
G102 / A701   Archive mutation
```

`G`, `A`, and `V` are ordering/watermark integers, not parent relationships. Conversation ancestry remains node-local. Profile identity and backing-object existence are not semantic timeline events.

## Invariants

- one physical CVA owner; one shared physical reopen scan;
- no generalized semantic database/root/dependency layer;
- Archive and vector-generation local clocks remain independent;
- each CVA-global version may be claimed by at most one semantic mutation across those owners;
- packed matrices, Archive-Vector bindings, and profiles are immutable backing objects;
- Archive Vectors own row-to-fragment identity only;
- profiles own embedding-space identity only;
- generations own profile-to-ArchiveVector association, coverage, activation, and vector semantic ordering;
- profile dimensions must match the generation's packed matrix;
- generation source watermark must cover every mapped fragment;
- latest generation per profile is current; different profiles remain independent.

## Code map

| Responsibility | Primary code |
| --- | --- |
| CVA composition/lifecycle | `src/cva.rs`, `src/cva_lifecycle.rs`, `src/cva_*` |
| physical Container/global clock | `src/container*.rs` |
| Archive/history/fragments | `src/archive*.rs`, `src/fragment*.rs` |
| packed matrices | `src/packed_vector_*.rs` |
| Archive row bindings | `src/archive_vector_*.rs` |
| endpoint/profile identity | `src/embedding_endpoint.rs`, `src/embedding_profile_*.rs` |
| generation publication/history | `src/vector_generation_*.rs` |
| corpus vector smoke | `examples/vector_generation_smoke.rs` |

## Related docs

- [Storage format](storage-format.md)
- [Rust API](api.md)
- [Architectural invariants](invariants.md)
- [Versioning and rollback plan](version-history-plan.md)
- [ADR 0006](decisions/0006-archive-vector-row-bindings.md)
- [ADR 0007](decisions/0007-embedding-profiles-and-vector-generations.md)

## Notes

Exact similarity search, live provider adapters, explicit generation retirement, and whole-CVA restore-and-continue remain separate slices.