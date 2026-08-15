# Rust API Reference

Parent index: [Documentation index](INDEX.md)

## Purpose

This document owns the current public Rust library surface exposed by `continuity_memory`.

## Overview

The API exposes explicit CVA composition, Archive operations, vector backing layers, embedding profiles, and vector-generation publication without a generalized semantic-store abstraction.

## Cva and Container

`Cva::create(path)` initializes all current concrete stores in one CVA. `Cva::open(path)` performs one physical scan and rebuilds them. `Cva::sync()` explicitly flushes the underlying Container.

`Container` remains public for physical-format testing and exposes opaque append/read operations, `ChunkRef`, format inspection, sync, and the CVA-global `latest_version()`.

## Archive

Public models include `ContentId`, `Node`, `Branch`, `ResolvedTurn`, `ArchiveStats`, `FragmentId`, `Fragment`, `FragmentConfig`, and `ArchiveRecordVersion`.

Important `Cva` Archive operations:

- `append_node(...)`;
- `append_branch(...)`;
- `branch_turns(...)`;
- `branch_at(..., archive_version)`;
- `materialize_path_fragments(...)`;
- `materialize_branch_fragments(...)`;
- `fragment_turns(...)` / `fragment_text(...)`;
- `fragments()` in deterministic `FragmentId` order;
- `archive_version()`, `record_versions()`, and `record_version(A)`.

`ArchiveRecordVersion { global_version, archive_version, record }` records ordering only. Conversation ancestry comes from node parents.

## Packed vectors

Public types:

```text
ScalarType
VectorSchema { dimensions, scalar }
PackedVectors
PackedVectorId
PackedVectorInfo
PackedVectorStats
PackedVectorError
```

`ScalarType` supports signed/unsigned 8/16/32/64-bit integers plus f16, bf16, f32, and f64. `VectorSchema` accepts any non-zero `u32` dimension count.

Operations:

- `put_packed_vectors(packed)`;
- `packed_vectors(id)`;
- `packed_vector_infos()`;
- `packed_vector_stats()`.

Packed matrices are immutable backing objects and consume no semantic version.

## Archive Vectors

Public types:

```text
ArchiveVectorId
ArchiveVectorSet { id, packed_vector_id, fragment_ids }
ArchiveVectorInfo {
    id,
    packed_vector_id,
    rows,
    max_fragment_archive_version,
}
ArchiveVectorStats
ArchiveVectorError
```

`put_archive_vectors(packed_vector_id, fragment_ids)` creates or reuses an immutable row binding. Row `N` maps to `fragment_ids[N]`. Matrix row count, fragment existence, uniqueness, and content identity are validated.

`max_fragment_archive_version` is derived reopen metadata used to validate generation coverage; it is not part of persistent Archive-Vector identity.

Operations also include `archive_vectors(id)`, `archive_vector_infos()`, and `archive_vector_stats()`.

## Embedding endpoints and profiles

Public endpoint types:

```text
EmbeddingEndpoint
EmbeddingEndpointDescriptor
EmbeddingEndpointError
EmbeddingMode::{Query, Document}
VectorNormalization::{None, L2}
SimulatedEmbeddingEndpoint
```

`EmbeddingEndpoint` exposes `descriptor()` and `embed(mode, inputs)`. The current built-in implementation is deterministic simulation for tests/development; no live provider adapter exists yet.

Public profile types:

```text
EmbeddingProfileId
EmbeddingProfile {
    id,
    provider,
    model,
    revision,
    dimensions,
    normalization,
    probe_suite_version,
    behavior_fingerprint,
}
EmbeddingProfileStats
EmbeddingProfileError
```

Profile operations:

- `create_embedding_profile(endpoint)` probes the endpoint and content-addresses the observed vector-space identity;
- `embedding_profile(id)`;
- `embedding_profiles()`;
- `embedding_profile_stats()`;
- `verify_embedding_endpoint(id, endpoint)` reruns the current probe suite and requires exact profile equality.

Profiles are immutable and clock-neutral. Two endpoints with identical advertised metadata but different observed probe vectors receive different profile IDs.

## Vector generations

Public types:

```text
VectorGenerationId
VectorGeneration {
    id,
    profile_id,
    archive_vector_id,
    source_archive_version,
    global_version,
    vector_version,
}
VectorGenerationStats
VectorGenerationError
```

### High-level development builder

`build_archive_vector_generation(profile_id, endpoint)`:

1. verifies the endpoint against the profile;
2. captures the current Archive watermark;
3. embeds every durable Archive fragment in deterministic `FragmentId` order using document mode;
4. validates count/dimensions/finiteness and declared normalization;
5. stores an `f32` packed matrix;
6. stores the Archive-Vector row binding;
7. publishes a VectorGeneration.

The method is deterministic with `SimulatedEmbeddingEndpoint`. Rebuilding an unchanged Archive with the same simulated endpoint is idempotent.

### Publication/history operations

- `publish_vector_generation(profile_id, archive_vector_id, source_archive_version)` publishes an already-built population after structural/reference validation;
- `vector_generation(id)`;
- `current_vector_generation(profile_id)`;
- `vector_generation_at(profile_id, vector_version)`;
- `vector_version()`;
- `vector_generation_stats()`.

Publication consumes one CVA-global version and one dense vector-local version. The newest generation for each profile is current. Source Archive versions cannot regress or predate mapped fragments.

Low-level `publish_vector_generation` cannot prove that externally supplied vector bytes truly came from the claimed endpoint; the high-level builder performs endpoint/profile verification.

## Defaults and errors

`FragmentConfig::default()` is eight turns with two-turn overlap. `CvaError` covers create/open composition across all current stores. Store-specific errors remain exposed for direct operations.

## Related docs

- [Architecture](architecture.md)
- [Storage format](storage-format.md)
- [Behavioral contracts](behavioral-contracts.md)
- [ADR 0007](decisions/0007-embedding-profiles-and-vector-generations.md)

## Notes

No compatibility promise has yet been made for Rust signatures or the development persistence format. Exact similarity search and live provider adapters are not implemented.