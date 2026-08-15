# Rust API Reference
Parent index: [Documentation index](INDEX.md)
## Purpose
This document owns the current public Rust library surface exposed by `continuity_memory`.
## Overview
The public API exposes `ContinuityConfig` and the initial model-switchboard types for machine-local runtime routing plus `Cva` for semantic storage/retrieval; Archive history; vector backing/publication; compatibility profiles; hybrid retrieval; simulated and direct OpenAI-ready embedding capabilities; chunk references; and errors. It remains development-stage.
## Exact contract
### Local configuration
`ContinuityConfig::new(path)` creates an in-memory default config, `ContinuityConfig::open(path)` loads `continuity.cfg`, and `save()` validates and atomically replaces the current file. Public fields are `fragments: FragmentConfig`, `retrieval: RetrievalConfig`, `models: ModelSwitchboardConfig`, and `credentials: CredentialsConfig`; `path()` reports the configured path. `load_or_create_master_key()` generates or reloads a 256-bit local master key from temporary sibling `continuity.master-key.json`. Credential objects are encrypted/decrypted automatically on save/open. Unknown framed objects are preserved across saves. Configuration is separate from `.cva` and has no semantic history.
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

Core operations through `Cva` include `append_node`, `append_branch`, `branch_turns`, `branch_at`, current `branches()`, fragment materialization/read methods, `stats`, `archive_version`, `record_versions`, `record_version`, deterministic `fragments()`, and `sync()`.

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

### Master key
Public types are `MasterKey`, `MasterKeyError`, `MasterKeyStore`, `JsonMasterKeyStore`, and `MASTER_KEY_BYTES = 32`. `JsonMasterKeyStore::load_or_create()` reads an existing version-1 JSON key or creates one from operating-system entropy without overwriting an existing file. `MasterKey` does not expose key bytes publicly and redacts its `Debug` representation. The JSON store is transitional and is not a production secret store.

### Credentials
Public types are `CredentialId`, `Credential`, `CredentialsConfig`, `CredentialError`, and redacting `SecretString`. `CredentialId::new` accepts non-empty ASCII alphanumeric/`.`/`_`/`-` IDs up to 128 bytes. `CredentialsConfig::insert_api_key` stores API-key auth in memory; `insert_chatgpt_oauth` stores ID/access/refresh tokens plus optional account ID. `get`, `remove`, and `ids` expose current credential inventory without exposing secret strings through `Debug`.

Credential objects are persisted as AES-256-GCM encrypted `credential.<id>` config objects. Wrong master keys and modified ciphertext are rejected during `ContinuityConfig::open`.

### Model switchboard
Public types are `ModelProvider::{OpenAiCodex, OpenAiReady}`, `ModelCapability::{General, Embedding}`, `ModelAuthKind::{ChatGptDeviceCode, ApiKey}`, `GeneralModelEndpoint`, `EmbeddingModelEndpoint`, `ModelSwitchboardConfig`, `ModelRequestAuth`, and `ModelSwitchboard`.

Each route includes a `CredentialId`. `ModelProvider::supports(capability)` exposes provider capability. `OpenAiCodex` currently supports General only, uses provider-owned routing, and requires ChatGPT OAuth material; `OpenAiReady` supports General and Embedding, requires explicit HTTP(S) URLs, and requires API-key material. Embedding routes also require non-zero dimensions and explicit normalization.

`ModelSwitchboard::new(config, credentials)` validates both route structure and credential availability/auth kind. `general_auth()` and `embedding_auth()` return redacting `ModelRequestAuth` values. `ModelRequestAuth::apply_to` adds `Authorization: Bearer ...` and, for ChatGPT auth when available, `ChatGPT-Account-ID`. Direct OpenAI-ready embedding transport is implemented; General-model HTTP transport and Codex device-code acquisition/token refresh remain future work.

### Embedding endpoint abstraction
Public types:

```text
EmbeddingMode::{Query, Document}
VectorNormalization::{None, L2}
EmbeddingEndpoint
EmbeddingEndpointError
SimulatedEmbeddingEndpoint
OpenAiReadyEmbeddingEndpoint
```

`EmbeddingEndpoint` reports dimensions and normalization and embeds a batch in either Query or Document mode. `SimulatedEmbeddingEndpoint` remains deterministic test plumbing. `OpenAiReadyEmbeddingEndpoint::from_switchboard` creates a live API-key endpoint from a validated embedding route; Query/Document map to `search_query`/`search_document`, requests use float encoding and the configured dimensions, and returned rows are restored by response index before optional L2 normalization.

`OpenAiReadyEmbeddingEndpoint` defaults to 16 inputs per HTTP batch and at most 16 concurrent requests; `with_batching(batch_size, concurrency)` overrides both with non-zero values. The 16×16 default is based on the 2026-08-15 live OpenRouter/Qwen3 embedding benchmark recorded in [development](development.md). `SimulatedEmbeddingEndpoint::new(dimensions, normalization, seed)` and `with_drift(value)` remain available for deterministic compatibility tests.

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

Provider, model, route, and revision are not compatibility-profile fields. Profile ID content-addresses exact stored reference evidence, but endpoint compatibility is tolerant cosine comparison. Compatibility policy v2 requires every corresponding probe to reach cosine `>= 0.9998` and requires dimensions, normalization, probe-suite version, and policy version to match. Policy v2 was calibrated against observed same-route drift from `qwen/qwen3-embedding-8b` through OpenRouter; broader endpoint calibration remains measurement-driven.

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
Public surface: `SemanticSearchHit { fragment, score, ordinal, generation_id }`, `SemanticSearchError`, and `MAX_SEMANTIC_SEARCH_LIMIT = 1000`.

`Cva::semantic_search(profile_id, endpoint, query, limit)` requires a non-empty query and limit `1..=1000`. It verifies the endpoint against the selected compatibility profile, embeds the query in `EmbeddingMode::Query`, resolves that profile's current Vector Generation, exact-scans its `f32` packed matrix using cosine similarity, drops scores `<= 0`, orders descending with row ordinal as the deterministic tie-break, and maps selected rows through Archive Vectors to durable `Fragment` records.

Each hit carries the generation ID actually searched. Search is read-only: it does not advance Archive, vector, or CVA-global semantic clocks. The current direct API verifies endpoint compatibility on every call; a future long-lived runtime may cache a verified capability without changing the generation/search ownership model.

No lexical search, hybrid score fusion, ANN index, reranker, or retrieval controller is part of this low-level method.

### Default hybrid retrieval
Public surface: `RetrievalConfig`, `SearchCandidate { fragment, lexical_score, semantic_score, combined_score, generation_id }`, `SearchError`, `DEFAULT_LEXICAL_WEIGHT = 0.45`, `DEFAULT_SEMANTIC_WEIGHT = 0.55`, `DEFAULT_SEARCH_CANDIDATE_LIMIT = 30`, and `DEFAULT_SEARCH_RESULT_LIMIT = 10`.

`Cva::search(profile_id, endpoint, query)` restores the original default search path. Lexical and semantic channels independently contribute up to 30 candidates. Candidates are merged by `FragmentId`; if only one channel produced a positive finite score, that score is retained without dilution. If both did, the combined score is `0.45 × lexical + 0.55 × semantic`.

Lexical scoring uses the original formula: `0.85 × unique query-term coverage + 0.15 × bounded matched-term frequency density`. Candidate ordering is followed by duplicate range removal and greedy diversity selection that multiplies same-conversation candidates by `1 - overlap` for each already-selected fragment. V2 computes overlap from actual node membership because fragments store node ranges rather than v1 numeric turn sequences. The diversified candidate pool is capped at 30 and the default method returns the first 10.

`Cva::search_with_config(profile_id, endpoint, query, config)` applies a validated `RetrievalConfig`; positive finite weights are normalized before fusion. `Cva::search` delegates to `RetrievalConfig::default()`. The current path has no reranker or conversation/time/metadata filters and remains read-only.

## Defaults or precedence
`FragmentConfig::default()` is eight turns with two-turn overlap. Compatibility probe suite v1 and compatibility policy v2 are fixed by the current implementation.

Node identity is `(conversation_id, node_id)`. Branch identity is `(conversation_id, branch_id)`. Repeated branch records are revisions; the newest Archive version is current.

## Diagnostics or failure behavior
`ConfigError`, `ArchiveError`, `PackedVectorError`, `ArchiveVectorError`, `CompatibilityProfileError`, `VectorGenerationError`, `SemanticSearchError`, and `SearchError` own their concrete domains. `CvaError` composes create/open failures and rejects global-version claims shared by multiple semantic owners.

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
- [Local configuration](configuration.md)
- [Repo-local CLI](cli.md)
- [Behavioral contracts](behavioral-contracts.md)
- [ADR 0007](decisions/0007-compatibility-profiles-and-vector-generations.md)
- [ADR 0008](decisions/0008-purpose-built-local-configuration.md)
- [ADR 0009](decisions/0009-expandable-model-switchboard.md)
- [ADR 0010](decisions/0010-encrypted-credential-objects.md)
- [ADR 0011](decisions/0011-detachable-repo-local-cli.md)

## Notes
No compatibility promise has yet been made for Rust method signatures or development persistence formats. The detachable CLI consumes this public surface only. OpenAI-ready embedding transport is implemented; General-model provider transport, Codex device-code acquisition/refresh, and the long-lived runtime remain next.
