# Rust API Reference
Parent index: [Documentation index](INDEX.md)
## Purpose
This document owns the current public Rust library surface exposed by `reliquary_memory`.
## Overview
The public API exposes `ReliquaryConfig` and model-switchboard types for machine-local runtime routing; `InteractionRuntime` for transport-neutral live session coordination, stream assembly, durable completed-turn acceptance, and explicit live Episode scheduling; `WorkspaceMetadata` for one CVA's durable workspace identity/type; and `Cva` for workspace lifecycle, semantic storage/retrieval, Archive history, Episodes, Memories, embedded files/source attachments, vector backing/publication, compatibility profiles, and hybrid retrieval. It remains development-stage.
## Exact contract
### Local configuration
`ReliquaryConfig::new(path)` creates an in-memory default config, `ReliquaryConfig::open(path)` loads `reliquary.cfg`, and `save()` validates and atomically replaces the current file. Public fields are `fragments: FragmentConfig`, `retrieval: RetrievalConfig`, `models: ModelSwitchboardConfig`, and `credentials: CredentialsConfig`; `path()` reports the configured path. `load_or_create_master_key()` generates or reloads a 256-bit local master key from temporary sibling `reliquary.master-key.json`. Credential objects are encrypted/decrypted automatically on save/open. Unknown framed objects are preserved across saves. Configuration is separate from `.cva` and has no semantic history.
### Container
Public types include `Container`, `ContainerError`, `ChunkRef { offset, len }`, and `FormatVersion`. Container exposes create/open, opaque append/read/chunk enumeration, sync, path/format inspection, and `latest_version()` for the CVA-global clock.
### Workspace metadata

Public types are `WorkspaceMetadata { id, name, workspace_type }`, `WorkspaceMetadataError`, and the field-size constants `MAX_WORKSPACE_ID_BYTES = 256`, `MAX_WORKSPACE_NAME_BYTES = 1024`, and `MAX_WORKSPACE_TYPE_BYTES = 256`.

`WorkspaceMetadata::new(id, name, workspace_type)` validates non-blank bounded UTF-8 fields. The workspace type is an extensible string identifier rather than a closed enum; Reliquary persists the type but does not own host capability composition.

`Cva::create_workspace(path, metadata)` creates a CVA and initializes its workspace metadata. `Cva::workspace_metadata()` returns the current metadata when present. `Cva::initialize_workspace_metadata(metadata)` upgrades an otherwise valid CVA exactly once; a second initialization is rejected. Ordinary `Cva::create` remains valid and returns a CVA with no workspace metadata so existing library workflows are not forced into a product workspace immediately.

Workspace metadata is clock-neutral: initialization consumes no Archive, Memory, Vector Generation, or CVA-global semantic version. Rename/type-change lifecycle is not exposed in this first slice.

### CVA comparison and reconciliation

Public reconciliation types are `CvaComparison`, `CvaRelation`, `CvaReconcileResult`, and `CvaReconcileError`. `Cva::compare(left, right)` opens and validates two CVAs, requires matching `WorkspaceMetadata.id` values, compares their physical append-only chunk histories, and reports `Identical`, `LeftExtendsRight`, `RightExtendsLeft`, or `Diverged` plus common/total chunk counts.

`Cva::reconcile(left, right, output)` writes a new output CVA. Identical/strict-extension cases copy the complete valid side. Diverged copies currently use the left CVA as the base and semantically replay the right divergent state through normal owner APIs so branch-local clocks are reallocated rather than copied. Replay now covers source Nodes, attachment-bearing ingested turns, standalone Files, Branch revisions, immutable Episodes, Memory revisions, durable Insomnia completion receipts, and file-to-Memory links. Fragment records remain rebuildable derived state and are omitted from the replay.

Memory replay preserves stable IDs and mutation IDs and uses ordinary expected-revision rules, so identical mutations deduplicate while incompatible concurrent revisions surface as Memory conflicts. Grouped Insomnia completions are decomposed into their embedded Memory revisions, those revisions are replayed normally, and a completion-only receipt is re-emitted against the merged Memory IDs. Identical completion receipts deduplicate; incompatible completions for the same Episode fail closed. Failed reconciliation output is removed. Derived vector/index rebuild and canonical-file promotion remain future work; see ADR 0018.

### Archive
Public Archive models include `ContentId`, `Node`, `Branch`, `ResolvedTurn`, `ConversationSummary`, `ArchiveStats`, `FragmentId`, `Fragment`, `FragmentConfig`, `FileId`, `StoredFile`, `IncomingAttachment`, `IncomingTurn`, `IngestedTurn`, `FileMemoryLink`, and:

```text
ArchiveRecordVersion {
    global_version: u64,
    archive_version: u64,
    record: ChunkRef,
}
```

Core operations through `Cva` include `append_node`, native `ingest_turn`, `append_branch`, `branch_turns`, `branch_at`, current `branches()`, derived `conversation_summaries()`, exact-leaf `conversation_turns(conversation_id, leaf_node_id)`, fragment materialization/read methods, `store_file`, `file`, `files`, `file_bytes`, `search_files`, `files_for_source`, `link_file_to_memory`, `file_memory_links`, `files_for_memory`, `stats`, `archive_version`, `record_versions`, `record_version`, deterministic `fragments()`, and `sync()`.

`ingest_turn(IncomingTurn)` is the source-ingestion boundary. `IncomingTurn` carries the source node plus zero or more `IncomingAttachment { filename, mime_type, bytes }` values. The node, attachment manifests, and source-to-file provenance are published as one Archive semantic mutation; callers do not store an attachment and then separately link it back to the turn. `files_for_source(conversation_id, node_id)` resolves those native attachments. Repeating the identical source event is idempotent; changing the attachment set for an existing node is a conflict.

`Cva::ingest_turn` remains the lower-level synchronous Archive-facing operation. `InteractionRuntime::accept_turn` is the current transport-neutral completed-turn runtime boundary described below. The development graph importer still calls `Cva::ingest_turn` directly because it is a batch import path rather than a live interaction adapter.

`store_file` remains the lower-level path for a file that is not being introduced as part of a source turn. `link_file_to_memory` is intentionally separate: a later Memory relationship is a real cross-owner semantic link, not source-ingestion provenance. File manifests carry filename, optional MIME type, byte length, and a content-addressed reference to arbitrary binary CAM bytes. One `CVACONT1` content object uses a `u32` byte-length field, so one stored file/content body must be smaller than 4 GiB. `search_files(query, limit)` uses the disposable incremental lexical index over filenames only; punctuation such as `-` and `.` separates terms, so extensions are searchable. File-tree operations and file-content indexing are not part of this surface yet.

`conversation_summaries()` derives one summary per conversation from current Archive nodes without publishing any record or choosing one branch. Each summary contains the conversation ID, all current durable leaf node IDs, turn count, and latest source timestamp. Leaf IDs are ordered newest-timestamp first with node ID as a deterministic tie-break. `conversation_turns` requires an explicit leaf and resolves exactly that ancestry path; callers must not silently collapse a multi-leaf conversation to one branch.

Global/Archive versions are ordering and watermarks only. Conversation ancestry remains node-parent based.

### Normalized interaction runtime

Public interaction types are `InteractionRole::{User, Agent}`, `InteractionAttachment`, `InteractionTurn`, `InteractionSession`, `InteractionReceipt`, `InteractionCompletion`, `InteractionError`, and `InteractionRuntime`.

`InteractionTurn` is the first transport-neutral completed-turn contract above Archive ingestion. It carries `message_id`, `session_id`, optional `parent_message_id`, normalized role, timestamp, content, and zero-or-more byte-bearing attachments. `session_id` maps to Archive `conversation_id`; message IDs map to node IDs; `User` maps to Archive role `user`; `Agent` maps to Archive role `assistant`. Protocol/provider-specific fields are intentionally absent from this contract.

`InteractionRuntime::new(cva)` takes ownership of one `Cva`. `open_session(session_id, resume_from)` establishes runtime session state. A new session uses `resume_from = None`; if durable Archive nodes already exist for that session ID, an explicit durable resume message is required. The resume message must exist in that same Archive conversation. `session()` reports the current durable leaf and any in-flight message; `close_session()` rejects closure while a message remains in progress.

`begin_message` starts one in-flight message whose parent is the session's current durable leaf. `append_text` appends text deltas, `attach` adds a complete attachment, `cancel_message` discards the in-flight buffer, and `complete_message` converts the assembled buffer into one `InteractionTurn`. In-flight buffers are runtime-only: they consume no Archive version and disappear if cancelled or if the runtime is lost before completion. A session permits only one in-flight message at a time.

`accept_turn(turn)` remains the direct completed-turn path. Both it and `complete_message` use the existing atomic source-turn ingestion semantics and call `Cva::sync()` before returning `InteractionReceipt { turn, archive_version }`. A successful receipt therefore means the source turn has crossed the current durability boundary. Identical completed-turn replay remains idempotent and does not advance the Archive watermark.

`complete_live_message(session_id, message_id, policy, now_ns)` is the normal live completion helper. It first performs `complete_message` and obtains the durable receipt, then attempts size-driven live Episode scheduling. It returns `InteractionCompletion { receipt, scheduling }`, where `scheduling` is an independent `Result`. A scheduling failure therefore cannot revoke or obscure an already-durable source receipt.

`schedule_session(policy, now_ns)` and `finalize_inactive_session(policy, now_ns)` also expose the existing live Episode/Insomnia scheduling paths from the current durable session leaf and sync any resulting semantic Episode publication. `finalize_inactive_session` remains an explicit call until a long-lived timer/service loop exists.

`conversation_summaries()` and `conversation_turns(conversation_id, leaf_node_id)` expose the same derived Archive inventory/path reads through the live runtime without giving hosts mutable CVA access. `cva()` provides read-only access to the owned CVA and `into_cva()` transfers ownership back to the caller. Automatic adapter reconnect/resume, process/service persistence, continuous timers, background Memory/vector execution, tool/session-event normalization, and IPC/API exposure remain future runtime work.

### Episodes

Public Episode models are `Episode`, `EpisodeId`, `EpisodeOrigin::{Live, Import}`, `EpisodeBoundary::{Size, Inactivity, CreateMemory, ImportEnd}`, `EpisodeConfig`, and `EpisodeBuildResult`. `DEFAULT_EPISODE_MAX_INPUT_BYTES` is 32 KiB and `DEFAULT_EPISODE_INACTIVITY_NS` is the current inactivity-policy default.

`materialize_branch_episodes` and `materialize_path_episodes` derive deterministic contiguous ancestry ranges from whole user-led response cycles. `episodes`, `episodes_for_conversation`, `episode`, and `episode_turns` inspect the resulting Archive-owned source ranges. Finalizing an Episode does not close its conversation.

### Memories

Public Memory models are `MemoryId`, `MemoryBodyId`, `MemoryRevisionId`, `MemoryDraft`, `Memory`, `MemoryStats`, and `MemoryError`.

`Cva::publish_memory(id, expected_revision, draft)` publishes one Memory revision after validating Archive/Episode provenance. A new Memory may omit `id`, in which case its stable ID derives from `mutation_id`; replaying the same mutation/draft is idempotent, while changed content under the same mutation conflicts. Existing Memory IDs require the exact expected current revision.

A Memory's semantic title/content is represented by one immutable `MemoryBodyId`. Later revisions may change metadata, provenance, lifecycle, archival, parent, or supersession state, but they cannot mutate semantic title/content in place. A semantic correction is therefore another Memory rather than a rewritten body.

`memory(id)`, `memory_revision(id, revision)`, `memory_stats()`, `memory_version()`, and `memory_body_id(id)` expose current/historical revision state and the dense Memory-local watermark. Memory publication consumes one CVA-global semantic version and one dense `memory_version`; Memory-body backing objects are content-addressed and clock-neutral.

`source_node_id` plus `source_episode_id` identify user authority inside an authoritative Episode. `content_source_*` identifies adopted/retained assistant authority when present; `grounding_source_*` identifies context used only to resolve a referent. Both external source tuples must resolve to real Archive nodes. Grounding never supplies semantic authority.

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

Credential objects are persisted as AES-256-GCM encrypted `credential.<id>` config objects. Wrong master keys and modified ciphertext are rejected during `ReliquaryConfig::open`.

### Model switchboard
Public types are `ModelProvider::{OpenAiCodex, OpenAiReady}`, `ModelCapability::{General, Insomnia, Embedding}`, `ModelAuthKind::{ChatGptDeviceCode, ApiKey}`, `ModelReasoningEffort::{None, Minimal, Low, Medium, High, XHigh, Max}`, `GeneralModelEndpoint`, `EmbeddingModelEndpoint`, `ModelSwitchboardConfig`, `ModelRequestAuth`, `ModelSwitchboard`, and `ConfiguredGeneralEndpoint`.

Each route includes a `CredentialId`. `ModelProvider::supports(capability)` exposes provider capability. `OpenAiCodex` supports General/Insomnia routing, uses provider-owned routing, requires ChatGPT OAuth material, and requires an explicit `ModelReasoningEffort`; `OpenAiReady` supports General/Insomnia/Embedding, requires explicit HTTP(S) URLs and API-key material, and currently leaves General reasoning unset. Embedding routes also require non-zero dimensions and explicit normalization. `models.insomnia` is optional and resolves to the General route/credential when absent.

`ModelSwitchboard::new(config, credentials)` validates route structure and credential availability/auth kind. `general_auth()`, `insomnia_auth()`, and `embedding_auth()` return redacting `ModelRequestAuth` values. `ModelRequestAuth::apply_to` adds `Authorization: Bearer ...` and, for ChatGPT auth when available, `ChatGPT-Account-ID`. `OpenAiCodexDeviceAuth` implements ChatGPT device-code acquisition against `auth.openai.com`, including authorization polling, OAuth code exchange, account-ID extraction, and insertion into `CredentialsConfig`. `ConfiguredGeneralEndpoint` dispatches the selected General/Insomnia route to either `OpenAiReadyGeneralEndpoint` or `OpenAiCodexGeneralEndpoint`; the latter uses the ChatGPT Codex Responses backend, structured JSON-schema output, SSE streaming, and the route's reasoning effort. OAuth token refresh remains future work.

### Insomnia extraction and backlog worker
Public Insomnia surface includes `InsomniaExtractor`, `InsomniaWorkerConfig`, `InsomniaDrainResult`, `InsomniaWorkerError`, queue/work/attempt models, and the extraction contract constants. `ConfiguredGeneralEndpoint::from_insomnia_switchboard` constructs the effective dedicated-Insomnia-or-General-fallback endpoint and dispatches it to the selected provider transport.

`Cva::finalize_canonical_imports_and_queue(EpisodeConfig, now_ns)` materializes uncovered deterministic import Episodes for canonical Archive branches and idempotently ensures all import-origin Episodes have Insomnia work. `Cva::drain_insomnia_backlog(extractor, embedding_endpoint, config)` runs a finite concurrent drain with 1–64 workers (default 48, calibrated against the current Luna/low corpus run). Claims, evidence reads, and authoritative Memory/attempt publication acquire the shared CVA owner; model calls execute outside that lock and therefore overlap across Episodes. Retryable extraction failures are requeued until `max_attempts`; invalid endpoint configuration is terminal. After all eligible Episode work is complete, the same core call establishes/reuses the embedding compatibility profile and automatically fills only missing `(CompatibilityProfileId, MemoryBodyId)` bindings. The result reports attempts, completed/terminal episodes, Memory outcomes, evidence volume, observed peak active worker count, and Memory-Vector fill details.

This worker is a finite bring-up/runtime primitive, not the future persistent shared Reliquary runtime. Memory publication remains authoritative before vectorization: if embedding/profile establishment fails, completed Memories and Insomnia outcomes remain in the CVA, and a later drain with no queued Episode work will retry the still-missing Memory Vectors. Reusing the same compatibility profile performs no re-embedding; a genuinely new profile creates new immutable bindings for the existing Memory bodies.

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
No compatibility promise has yet been made for Rust method signatures or development persistence formats. The detachable CLI consumes this public surface only. Product/runtime gaps are tracked in [Current limitations](current-limitations.md), and future sequencing is tracked in [Roadmap](roadmap.md).
