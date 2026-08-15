# Rust API Reference

Parent index: [Documentation index](INDEX.md)

## Purpose
This document owns the current public Rust library surface exposed by `continuity_memory`.

## Overview
The public API exposes `Cva` as the physical composition owner; Archive history; packed-vector backing objects; Archive-Vector row bindings; compatibility profiles; vector-generation publication/history; exact semantic retrieval; simulated embedding endpoints; chunk references; and errors. It remains development-stage.

## Exact contract

### Container
Public types include `Container`, `ContainerError`, `ChunkRef { offset, len }`, and `FormatVersion`. Container exposes create/open, opaque append/read/chunk enumeration, sync, path/format inspection, and `latest_version()` for the CVA-global clock.

### Archive
Public Archive models include `ContentId`, `Node`, `Branch`, `ResolvedTurn`, `ArchiveStats`, `FragmentId`, `Fragment`, `FragmentConfig`, and:

```text
ArchiveRecordVersion {
    global_version: u64,
    archive_version: u64,
    record: ChunkRef,
}
```

Core operations through `Cva` include `append_node`, `append_branch`, `branch_turns`, `branch_at`, fragment materialization/read methods, `stats`, `archive_version`, `record_versions`, `record_version`, deterministic `fragments()`, and `sync()`.

Global/Archive versions are ordering and watermarks only. Conversation ancestry remains node-parent based.

### Packed vectors
Public types:

```text
ScalarType
VectorSchema
PackedVectors
PackedVectorId
PackedVectorInfo
PackedVectorStats
PackedVectorError
```

`Cva::put_packed_vectors(packed)` content-addresses and deduplicates an immutable matrix. `packed_vectors(id)` reads it; `packed_vector_infos()` and `packed_vector_stats()` inspect inventory. Packed matrices have no row-to-domain or compatibility semantics and consume no semantic clock.

### Archive Vectors
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

`Cva::put_archive_vectors(packed_vector_id, fragment_ids)` creates/reuses an immutable row binding. Packed row `N` maps to `fragment_ids[N]`. Matrix existence, exact row count, real fragments, and per-set fragment uniqueness are required.

`max_fragment_archive_version` is derived validation metadata, not Archive-Vector persistent identity. Archive Vectors contain no compatibility profile or active-generation state.

### Embedding endpoint abstraction
Public types:

```text
EmbeddingMode::{Query, Document}
VectorNormalization::{None, L2}
EmbeddingEndpoint
EmbeddingEndpointError
SimulatedEmbeddingEndpoint
```

`EmbeddingEndpoint` reports dimensions and normalization and embeds a batch in either Query or Document mode. The repository currently ships only `SimulatedEmbeddingEndpoint` for deterministic tests/building; production provider adapters are not implemented.

`SimulatedEmbeddingEndpoint::new(dimensions, normalization, seed)` creates a deterministic endpoint. `with_drift(value)` introduces deterministic small numeric drift for compatibility tests.

### Compatibility profiles
Public types/constants:

```text
CompatibilityProfileId
CompatibilityProbeReference { mode, vector }
CompatibilityProfile {
    id,
    dimensions,
    normalization,
    probe_suite_version,
    compatibility_policy_version,
    references,
}
CompatibilityReport { compatible, minimum_cosine }
CompatibilityProfileStats
CompatibilityProfileError
COMPATIBILITY_PROBE_SUITE_VERSION
COMPATIBILITY_POLICY_VERSION
COMPATIBILITY_MIN_COSINE
```

`Cva::establish_compatibility_profile(endpoint)` probes Query and Document behavior. If the candidate is tolerantly compatible with an existing profile, that existing profile is returned; otherwise a new immutable profile is stored.

`compatibility_profile(id)`, `compatibility_profiles()`, and `compatibility_profile_stats()` inspect stored contracts. `verify_compatibility_endpoint(id, endpoint)` returns a `CompatibilityReport` without creating state.

Provider, model, route, and revision are not compatibility-profile fields. Profile ID content-addresses exact stored reference evidence, but endpoint compatibility is tolerant cosine comparison. Current policy requires every corresponding probe to reach cosine `>= 0.99999` and requires dimensions, normalization, probe-suite version, and policy version to match.

Compatibility-profile creation consumes no semantic version ticket.

### Vector generations
Public types:

```text
VectorGenerationId
VectorGeneration {
    id,
    compatibility_profile_id,
    archive_vector_id,
    source_archive_version,
    global_version,
    vector_version,
}
VectorGenerationStats
VectorGenerationError
```

`build_archive_vector_generation(profile_id, endpoint)` verifies endpoint compatibility, embeds all current durable fragments in deterministic `FragmentId` order using Document mode, stores an `f32` matrix plus Archive-Vector binding, and publishes the generation.

`publish_vector_generation(profile_id, archive_vector_id, source_archive_version)` publishes already-created backing state after cross-store validation.

`vector_generation(id)`, `current_vector_generation(profile_id)`, `vector_generation_at(profile_id, vector_version)`, `vector_version()`, and `vector_generation_stats()` expose current/historical generation state.

Generation publication consumes one CVA-global semantic ticket and one dense local vector version. The latest generation per compatibility profile is current. Unversioned generation payloads are inert on reopen. Published generations currently require `f32` matrices; packed backing storage remains generic, but alternate searchable representations wait for explicit quantization/dequantization semantics.

### Exact semantic retrieval
Public types/constants:

```text
SemanticSearchHit {
    fragment,
    score,
    ordinal,
    generation_id,
}
SemanticSearchError
MAX_SEMANTIC_SEARCH_LIMIT = 1000
```

`Cva::semantic_search(profile_id, endpoint, query, limit)` requires a non-empty query and limit `1..=1000`. It verifies the endpoint against the selected compatibility profile, embeds the query in `EmbeddingMode::Query`, resolves that profile's current Vector Generation, exact-scans its `f32` packed matrix using cosine similarity, drops scores `<= 0`, orders descending with row ordinal as the deterministic tie-break, and maps selected rows through Archive Vectors to durable `Fragment` records.

Each hit carries the generation ID actually searched. Search is read-only: it does not advance Archive, vector, or CVA-global semantic clocks. The current direct API verifies endpoint compatibility on every call; a future long-lived runtime may cache a verified capability without changing the generation/search ownership model.

No lexical search, hybrid score fusion, ANN index, reranker, or retrieval controller is part of this method.

## Defaults or precedence
`FragmentConfig::default()` is eight turns with two-turn overlap. Compatibility probe suite/policy v1 are fixed by the current implementation.

Node identity is `(conversation_id, node_id)`. Branch identity is `(conversation_id, branch_id)`. Repeated branch records are revisions; the newest Archive version is current.

## Diagnostics or failure behavior
`ArchiveError`, `PackedVectorError`, `ArchiveVectorError`, `CompatibilityProfileError`, `VectorGenerationError`, and `SemanticSearchError` own their concrete domains. `CvaError` composes create/open failures and rejects global-version claims shared by multiple semantic owners.

Durability remains explicit through `Cva::sync()`.

## Examples
A compatibility profile may be established using one endpoint execution and later accept a slightly drifting execution when all probe similarities remain inside policy. A materially different endpoint cannot build a generation under that profile.

Archive and vector semantic clocks may interleave:

```text
G100/A700  Archive mutation
G101/V20   vector generation
G102/A701  Archive mutation
```

## Related docs
- [Architecture](architecture.md)
- [Storage format](storage-format.md)
- [Behavioral contracts](behavioral-contracts.md)
- [ADR 0007](decisions/0007-compatibility-profiles-and-vector-generations.md)

## Notes
No compatibility promise has yet been made for Rust method signatures or development persistence formats. Exact semantic retrieval is implemented; lexical/hybrid retrieval and production runtime integration remain later slices.
