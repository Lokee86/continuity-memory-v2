# Rust API Reference
Parent index: [Documentation index](INDEX.md)
## Purpose
This document owns the current public Rust library surface exposed by `reliquary_memory`.
## Overview
The public API exposes `ReliquaryConfig` and model-switchboard types for machine-local runtime routing; `InteractionRuntime` for transport-neutral live session coordination; `WorkspaceMetadata` for one Reliquary's durable workspace identity/type; `Reliquary` for the existing full non-user scope storage/runtime model; and the separate `Phylactery` type for user-global Identity state. Reliquary includes Archive history, Episodes, Memories, Graph, Insomnia state, embedded files/source attachments, vector backing/publication, compatibility profiles, reconciliation, and hybrid retrieval. Phylactery currently exposes source-independent Memories, Graph, Packed Vectors, Memory Vectors, and Compatibility Profiles. `Cva` remains exported as the Reliquary compatibility/internal type name. The API remains development-stage.
## Exact contract
### Local configuration
`ReliquaryConfig::new(path)` creates an in-memory default config, `ReliquaryConfig::open(path)` loads `reliquary.cfg`, and `save()` validates and atomically replaces the current file. Public fields are `fragments: FragmentConfig`, `retrieval: RetrievalConfig`, `models: ModelSwitchboardConfig`, and `credentials: CredentialsConfig`; `path()` reports the configured path. `load_or_create_master_key()` generates or reloads a 256-bit local master key from temporary sibling `reliquary.master-key.json`. Credential objects are encrypted/decrypted automatically on save/open. Unknown framed objects are preserved across saves. Configuration is separate from `.rel` and has no semantic history.
### Container
Public types include `Container`, `ContainerError`, `ChunkRef { offset, len }`, `FormatVersion`, `ContainerIdentity`, `FileKind`, and `ReliquaryScopeKind`. `FileKind` is `Reliquary` or `Phylactery`. New Reliquary creation uses typed identity plus Organization/Project/Connection scope; `Phylactery::create` uses typed Phylactery identity with no Reliquary scope. Invalid kind/scope combinations are rejected. Legacy 16-byte CVA headers remain distinguishable and are Project Reliquary only. `Reliquary::scope_kind()` reports Organization, Project, or Connection and `Reliquary::is_legacy_cva()` reports the legacy physical form. Container still exposes opaque append/read/chunk enumeration, sync, path/format inspection, and `latest_version()` for the existing file-global semantic clock.
### Workspace metadata

Public types are `WorkspaceMetadata { id, name, workspace_type }`, `WorkspaceMetadataError`, and the field-size constants `MAX_WORKSPACE_ID_BYTES = 256`, `MAX_WORKSPACE_NAME_BYTES = 1024`, and `MAX_WORKSPACE_TYPE_BYTES = 256`.

`WorkspaceMetadata::new(id, name, workspace_type)` validates non-blank bounded UTF-8 fields. The workspace type is an extensible string identifier rather than a closed enum; Reliquary persists the type but does not own host capability composition.

`Reliquary::create(path)` and `create_project(path)` create typed Project REL files; `create_organization(path)` and `create_connection(path)` create the other accepted typed Reliquary scopes. `open_project`, `open_organization`, and `open_connection` enforce the requested scope. `Reliquary::create_workspace(path, metadata)` initializes workspace metadata, while `workspace_metadata()` and `initialize_workspace_metadata(metadata)` retain the existing behavior. Legacy `.cva` files open as Project Reliquaries without an automatic header rewrite.

Workspace metadata is clock-neutral: initialization consumes no Archive, Memory, Vector Generation, or CVA-global semantic version. Rename/type-change lifecycle is not exposed in this first slice.

### CVA comparison and reconciliation

Public reconciliation types are `CvaComparison`, `CvaRelation`, `CvaReconcileResult`, `CvaReconcileConflict`, and `CvaReconcileError`. `Cva::compare(left, right)` opens and validates two CVAs, requires matching `WorkspaceMetadata.id` values, compares their physical append-only chunk histories, and reports `Identical`, `LeftExtendsRight`, `RightExtendsLeft`, or `Diverged` plus common/total chunk counts.

`Cva::reconcile(left, right, output)` writes a new output CVA. Identical/strict-extension cases copy the complete valid side. True divergence is handled as a fresh semantic repack: Reliquary creates a new workspace CVA, replays the complete left semantic history plus the right divergent tail through normal owner APIs, and therefore allocates fresh valid global/Archive/Memory/Graph clocks rather than retaining either branch's physical/version layout. Replay covers source Nodes, attachment-bearing ingested turns, standalone Files, Branch revisions, immutable Episodes, immutable Fragments, Memory revisions, Graph relationship transactions, durable Insomnia completion receipts, file-to-Memory links, interaction-stream checkpoints, and compatibility profiles.

Memory replay preserves stable IDs and mutation IDs and uses ordinary expected-revision rules, so identical mutations deduplicate while incompatible concurrent revisions surface as Memory conflicts. Graph replay occurs only after the relevant Memories exist. Single-edge and atomic `GraphRelationChange` batches are decoded from their source histories and republished through the Graph owner so destination global/Graph versions and dense topology mappings are allocated afresh. For each oriented `(source, target, kind)` relationship, left/right divergent active-state sequences are compared by semantic prefix: an already-present prefix is deduplicated, a stale candidate contributes nothing, and only an unabsorbed right suffix is replayed. A non-prefix history fails closed as `CvaReconcileConflict::GraphRelation`; with the current binary toggle model, valid histories from one shared base are naturally prefix-comparable. Grouped Insomnia completions are decomposed into their embedded Memory revisions, those revisions are replayed normally, and a completion-only receipt is re-emitted against the merged Memory IDs. Existing Fragment ranges are revalidated and replayed, so the disposable lexical index rebuilds lazily from the merged Fragment/File population after reopen. Graph topology and Dream's duplicate index are derived state and rebuild from merged relationship authority rather than being reconciled as separate persistent indexes. Packed vector matrices, Memory-Vector bindings, Archive-Vector bindings, and Vector Generations are intentionally not copied into a divergent repack because their population cuts may be stale; compatibility profiles are retained so callers can rebuild them with a verified embedding endpoint. `CvaReconcileResult` reports replayed/duplicate Graph mutations and transactions as well as `canonical_change_required`. If a physically divergent candidate is already semantically absorbed, reconciliation restores an exact copy of the left/canonical CVA as output, reports no canonical change, and does not require vector rebuilding. `vector_rebuild_required` is therefore true only when a real divergent merge changed canonical state and retired derived vector data.

Semantic merge conflicts are returned as `CvaReconcileError::Conflict(CvaReconcileConflict)`. The public conflict variants currently identify source-turn collisions, divergent Branch heads (including existing/incoming leaf IDs), Episode/Fragment/File identity conflicts, Memory revision/mutation/semantic-mutation conflicts, incompatible Graph relation histories, and incompatible Insomnia completions (including the competing extractor model/version receipts). `CvaReconcileConflict::kind()` exposes a stable machine-facing kind string and `CvaReconcileError::conflict()` returns the structured conflict when present. Corruption, invalid format, I/O, and promotion failures remain ordinary non-conflict errors.

`Cva::reconcile_and_promote(canonical, conflicted)` adds the path-based safe-promotion lifecycle. It first reconciles into a hidden sibling candidate. If reconciliation reports `canonical_change_required = false`, the candidate is removed and the canonical bytes are left untouched. Otherwise promotion fingerprints the canonical CVA, syncs and validates the candidate, verifies that the canonical file did not change during preparation, writes and syncs a recovery copy, atomically replaces the canonical path, and reopens/syncs the promoted CVA before retiring the recovery copy. Reconciliation or promotion failure leaves the conflicted source untouched; any post-replacement finalization failure restores and revalidates the recovery copy before an error is returned. Hidden candidate/recovery files are cleaned up after normal success or failure. This is a filesystem-path primitive, not provider-specific conflicted-copy discovery or a general interprocess locking system; see ADR 0019.

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

Public interaction types are `InteractionRole::{User, Agent}`, `InteractionAttachment`, `InteractionTurn`, `InteractionSession`, `InteractionReceipt`, `InteractionCompletion`, `InteractionStreamRecord`, `InteractionStreamStatus`, `InteractionTurnStatus`, `ResolvedInteractionTurn`, `InteractionError`, and `InteractionRuntime`.

`InteractionTurn` is the first transport-neutral completed-turn contract above Archive ingestion. It carries `message_id`, `session_id`, optional `parent_message_id`, normalized role, timestamp, content, and zero-or-more byte-bearing attachments. `session_id` maps to Archive `conversation_id`; message IDs map to node IDs; `User` maps to Archive role `user`; `Agent` maps to Archive role `assistant`. Protocol/provider-specific fields are intentionally absent from this contract.

`InteractionRuntime::new(cva)` takes ownership of one `Cva`. `open_session(session_id, resume_from)` establishes runtime session state. A new session uses `resume_from = None`; if durable Archive nodes already exist for that session ID, an explicit durable resume message is required. The resume message must exist in that same Archive conversation. `session()` reports the current durable leaf and any in-flight message; `close_session()` rejects closure while a message remains in progress.

`begin_message` starts one in-flight message whose parent is the session's current durable Archive leaf. `append_text` appends runtime-only text, while `append_checkpointed_text` appends a delta and synchronously publishes the cumulative stream state into the append-only interaction-stream journal before returning. `checkpoint_message` persists the current cumulative in-flight text without adding another delta. `attach` adds a complete attachment, `cancel_message` discards an unacknowledged in-flight buffer, `interrupt_message` publishes the last assembled text with `Interrupted` state and ends the in-flight message, and `complete_message` converts the assembled buffer into one authoritative `InteractionTurn`. A session permits only one in-flight message at a time.

The interaction-stream journal is durable but is not an Archive semantic owner. It exists to retain user-visible partial output without promoting an incomplete assistant response into Archive/Episode/Insomnia authority. Stream checkpoints consume container chunks but no Archive, Memory, Vector Generation, or CVA-global semantic version. A process restart has no live in-memory stream, so a recovered `Streaming` record is exposed through transcript resolution as `Interrupted`. When `complete_message` successfully publishes the same message ID into Archive, transcript resolution prefers the completed Archive node and suppresses the journal copy.

`accept_turn(turn)` remains the direct completed-turn path. Both it and `complete_message` use the existing atomic source-turn ingestion semantics and call `Cva::sync()` before returning `InteractionReceipt { turn, archive_version }`. A successful receipt therefore means the source turn has crossed the current durability boundary. Identical completed-turn replay remains idempotent and does not advance the Archive watermark.

`complete_live_message(session_id, message_id, policy, now_ns)` is the normal live completion helper. It first performs `complete_message` and obtains the durable receipt, then attempts size-driven live Episode scheduling. It returns `InteractionCompletion { receipt, scheduling }`, where `scheduling` is an independent `Result`. A scheduling failure therefore cannot revoke or obscure an already-durable source receipt.

`schedule_session(policy, now_ns)` and `finalize_inactive_session(policy, now_ns)` also expose the existing live Episode/Insomnia scheduling paths from the current durable session leaf and sync any resulting semantic Episode publication. `finalize_inactive_session` remains an explicit call until a long-lived timer/service loop exists.

`conversation_summaries()` and `conversation_turns(conversation_id, leaf_node_id)` expose the same derived Archive inventory/path reads through the live runtime. `conversation_transcript(conversation_id, leaf_node_id)` overlays matching uncompleted interaction-stream records on that exact durable path, reports each turn as `Complete`, `Streaming`, or `Interrupted`, and keeps interrupted assistant text available to a later provider request without changing Archive authority. `cva()` provides read-only access to the owned CVA and `into_cva()` transfers ownership back to the caller. Automatic adapter reconnect/resume, continuous timers, background Memory/vector execution, tool/session-event normalization, and IPC/API exposure remain future runtime work.

### Episodes

Public Episode models are `Episode`, `EpisodeId`, `EpisodeOrigin::{Live, Import}`, `EpisodeBoundary::{Size, Inactivity, CreateMemory, ImportEnd}`, `EpisodeConfig`, and `EpisodeBuildResult`. `DEFAULT_EPISODE_MAX_INPUT_BYTES` is 32 KiB and `DEFAULT_EPISODE_INACTIVITY_NS` is the current inactivity-policy default.

`materialize_branch_episodes` and `materialize_path_episodes` derive deterministic contiguous ancestry ranges from whole user-led response cycles. `episodes`, `episodes_for_conversation`, `episode`, and `episode_turns` inspect the resulting Archive-owned source ranges. Finalizing an Episode does not close its conversation.

### Phylactery

`Phylactery::create(path)` creates a typed `.phy` with user-global semantic identity; `Phylactery::open(path)` requires that exact identity and rejects REL and legacy CVA files. The implemented owner composition is Memories, Graph, Packed Vectors, Memory Vectors, and Compatibility Profiles. Archive, Episodes, Files/attachments, Insomnia, Archive Vectors, Vector Generations, Workspace Metadata, interaction-stream checkpoints, and the current Archive-specific lexical index are not part of the Phylactery API.

Phylactery exposes Memory reads/revisions/stats/body IDs and `publish_memory`; Graph mutation/query APIs matching the same-file Reliquary Graph surface; packed-vector object APIs; compatibility-profile establishment/verification; and Memory-vector binding/build APIs including `build_missing_memory_vectors`. `sync()` flushes the shared container.

Current direct `Phylactery::publish_memory` requires a source-independent `MemoryDraft`: all existing REL-local Episode/node/conversation provenance fields must be `None`. Cross-file source export/lineage is not encoded by this API yet.

### Memories

Public Memory models are `MemoryId`, `MemoryBodyId`, `MemoryRevisionId`, `MemoryDraft`, `Memory`, `MemoryStats`, and `MemoryError`.

`Cva::publish_memory(id, expected_revision, draft)` publishes one Reliquary Memory revision after validating Archive/Episode provenance. `Phylactery::publish_memory` uses the same revision/body/mutation semantics but validates that REL-local provenance is absent. A new Memory may omit `id`, in which case its stable ID derives from `mutation_id`; replaying the same mutation/draft is idempotent, while changed content under the same mutation conflicts. Existing Memory IDs require the exact expected current revision.

A Memory's semantic title/content is represented by one immutable `MemoryBodyId`. Later revisions may change metadata, provenance, lifecycle, archival, parent, or supersession state, but they cannot mutate semantic title/content in place. A semantic correction is therefore another Memory rather than a rewritten body.

`memory(id)`, `memory_revision(id, revision)`, `memory_stats()`, `memory_version()`, and `memory_body_id(id)` expose current/historical revision state and the dense Memory-local watermark. Memory publication consumes one CVA-global semantic version and one dense `memory_version`; Memory-body backing objects are content-addressed and clock-neutral.

### Graph

Public Graph models are `GraphRelationKind`, `GraphDirection`, `GraphRelationChange`, `GraphRelation`, `GraphNeighbor`, `MemoryGraphPath`, `GraphStats`, and `GraphError`. Relationship kinds are `Topical`, `Factual`, `Causal`, `Recurrent`, `References`, `DuplicateOf`, `Supersedes`, and `StructuralParent`.

`Cva::set_memory_relation` / `set_memory_relations` publish or retract oriented Memory-to-Memory relationships inside a Reliquary. `Phylactery` exposes the same Graph mutation/query surface for Memories owned by that PHY. Both endpoints must already exist in the same file's Memory store, self-relations and duplicate identities within one batch are rejected, and the caller must supply the exact current `graph_version`. Already-visible states are removed as idempotent no-ops before publication. One non-empty transaction consumes one file-global version and one dense Graph-local version regardless of how many relationship changes it contains. The current endpoint format is `MemoryId` only and does not represent cross-file Graph edges.

`graph_version()`, `graph_stats()`, `graph_relations()`, `graph_neighbors(memory_id, direction)`, and `shortest_memory_path(source, target, max_depth)` expose current topology. Dense topology IDs are internal; all public endpoints and results use stable `MemoryId` values. Traversal delegates to the pinned repository-agnostic `arcana-graph` kernel while CVA persistence/versioning remains Reliquary-owned.

`source_node_id` plus `source_episode_id` identify user authority inside an authoritative Episode. `content_source_*` identifies adopted/retained assistant authority when present; `grounding_source_*` identifies context used only to resolve a referent. Both external source tuples must resolve to real Archive nodes. Grounding never supplies semantic authority.

### Dream candidate retrieval

Public types are `DreamCandidateConfig`, `DreamMemoryContext`, `DreamCandidate`, `DreamCandidateSet`, `DreamCandidateError`, and the `DEFAULT_DREAM_*` / `MAX_DREAM_CANDIDATE_LIMIT` constants.

`Cva::dream_candidates(profile_id, source_id, config)` performs bounded read-only candidate discovery for one existing Memory. The source Memory must already have a Memory-Vector binding under the selected compatibility profile. Its stored Document vector is compared directly with other stored Memory-body vectors, so this API performs no embedding endpoint or model call. A candidate without a vector may still enter through deterministic lexical/metadata or temporal matching.

`Cva::dream_memory_context(memory_id)` exposes the same production context builder used by candidate discovery for one exact Memory without ranking or requiring a vector. It returns the current Memory/body identity, authoritative source timestamp when resolvable, deterministic temporal analysis, and active Graph relations touching the Memory. This is a read-only inspection/tuning seam; it does not evaluate a relationship or mutate semantic state.

The current default limits are 12 final candidates, 24 primary semantic candidates, 3 prior-semantic reservations, 8 lexical candidates, and 8 temporal candidates. Prior-semantic coverage uses authoritative source timestamps rather than Memory creation timestamps. Temporal matching uses overlapping deterministic content-time anchors or matching recurrence patterns; source-time proximity alone is not a match. Archived Memories are excluded. Every returned source/candidate context includes the immutable `MemoryBodyId`, resolved authoritative source timestamp when available, deterministic temporal analysis, and all active Graph relations touching that Memory. Candidate discovery is deterministic for unchanged CVA state and configuration and does not mutate Graph, Memories, or semantic clocks.

### Dream deterministic temporal analysis

Public types are `DreamTemporalAnalysis`, `DreamTemporalAnchor`, `DreamTemporalPattern`, `DreamTemporalMatch`, `DreamTemporalGranularity`, `DreamTemporalOrigin`, `DreamTemporalFrequency`, `DreamTemporalWeekday`, and `DreamTemporalMatchKind`.

`Cva::dream_temporal_analysis(memory_id)` derives temporal context from the current immutable Memory body plus authoritative source chronology. Explicit parsing covers RFC3339 timestamps, ISO/natural calendar dates, inclusive date ranges, months, quarters, contextual years, and recurrence patterns. Relative forms such as `today`, `tomorrow`, `last week`, `next month`, `next Tuesday`, `in N days/weeks`, and `N days/weeks ago` are resolved only when authoritative source time is available. `Memory.created_at_ns` is never a temporal reference frame.

Temporal anchors are half-open nanosecond ranges and remain distinct from source chronology. Current calendar interpretation uses UTC because source provenance does not yet carry an explicit source timezone. Temporal analysis is derived/read-only: it has no CVA record family, version clock, or model-enrichment call. `DreamMemoryContext` carries the derived analysis into classification/verification payloads, where it is evidence rather than proof of causality, supersession, or recurrence.

### Dream pair classification

Public types are `DreamClassifier<E>`, `DreamPairClassification`, `DreamPairEvidence`, `DreamRelationKind`, `DreamRelationDirection`, `DreamEvidenceSide`, `DreamClassificationError`, `DREAM_CLASSIFIER_CONTRACT_VERSION`, `DREAM_CLASSIFIER_SYSTEM_PROMPT`, and `dream_classifier_schema()`.

`DreamClassifier::new(endpoint)` binds the production `DREAM_CLASSIFIER_SYSTEM_PROMPT`. `DreamClassifier::with_system_prompt(endpoint, prompt)` is the explicit tuning seam used by the exact-pair harness to measure a candidate without changing the production default. `DreamClassifier::classify_pair(left, right)` canonicalizes the two distinct Memory contexts into stable MemoryId A/B order, submits the canonical pair through `GeneralEndpoint::complete_json`, and validates the structured result. `classify_candidates(set)` evaluates every candidate in the already-bounded `DreamCandidateSet`; the classifier does not discover additional candidates.

The v2 classifier returns one primary proposal: `None`, `Topical`, `Factual`, `Causal`, `Recurrent`, `DuplicateOf`, or `Supersedes`. `Topical`, `Recurrent`, and `DuplicateOf` are semantically undirected; `Factual`, `Causal`, and `Supersedes` require an explicit A→B or B→A direction. Every non-none result requires exactly two verbatim evidence quotes, one from each Memory; invalid direction combinations or invented evidence are rejected. `None` requires no evidence. The result records the endpoint model name but is transient: classification does not write Graph state or change Memory lifecycle.

### Dream independent verification

Public types are `DreamVerifier<E>`, `DreamPairVerification`, `DreamVerificationPolicy`, `DreamVerificationSignal::{Yes, No, Uncertain}`, `DreamVerificationVerdict::{Accept, Reject, Uncertain}`, `DreamVerificationError`, `DREAM_VERIFIER_CONTRACT_VERSION`, `DREAM_VERIFIER_SYSTEM_PROMPT`, and `dream_verifier_schema()`.

`DreamVerifier::verify_pair(classification, left, right)` requires a non-`None` classification, canonicalizes the supplied contexts with the same MemoryId ordering used by the classifier, verifies that the classification identifies that exact pair, and submits the pair plus proposal/evidence through a separate strict structured-output call. The verifier returns categorical support for the proposed relation, direction, and classifier evidence. Any `No` deterministically yields `Reject`; otherwise any `Uncertain` yields `Uncertain`; three `Yes` signals yield `Accept`.

`verify_if_required(policy, ...)` skips the model call when the selected policy does not require verification. `DreamVerificationPolicy::default()` verifies `DuplicateOf` and `Supersedes`; `broad_semantic()` additionally verifies `Factual`, `Causal`, and `Recurrent` for measurement. Neither policy verifies `None`, and topical remains single-pass. Verification exposes no numeric confidence and remains transient: it writes no Graph state and changes no Memory lifecycle.

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
Public types are `ModelProvider::{OpenAiCodex, OpenAiReady}`, `ModelCapability::{General, Insomnia, Dream, Embedding}`, `ModelAuthKind::{ChatGptDeviceCode, ApiKey}`, `ModelReasoningEffort::{None, Minimal, Low, Medium, High, XHigh, Max}`, `GeneralModelEndpoint`, `EmbeddingModelEndpoint`, `ModelSwitchboardConfig`, `ModelRequestAuth`, `ModelSwitchboard`, and `ConfiguredGeneralEndpoint`.

Each route includes a `CredentialId`. `ModelProvider::supports(capability)` exposes provider capability. `OpenAiCodex` supports General/Insomnia/Dream routing, uses provider-owned routing, requires ChatGPT OAuth material, and requires an explicit `ModelReasoningEffort`; `OpenAiReady` supports General/Insomnia/Dream/Embedding, requires explicit HTTP(S) URLs and API-key material, and currently leaves General-model reasoning unset. Embedding routes also require non-zero dimensions and explicit normalization. `models.insomnia` and `models.dream` are optional and independently resolve to the General route/credential when absent.

`ModelSwitchboard::new(config, credentials)` validates route structure and credential availability/auth kind. `general_auth()`, `insomnia_auth()`, `dream_auth()`, and `embedding_auth()` return redacting `ModelRequestAuth` values. `ModelRequestAuth::apply_to` adds `Authorization: Bearer ...` and, for ChatGPT auth when available, `ChatGPT-Account-ID`. `OpenAiCodexDeviceAuth` implements ChatGPT device-code acquisition against `auth.openai.com`, including authorization polling, OAuth code exchange, account-ID extraction, and insertion into `CredentialsConfig`. `ConfiguredGeneralEndpoint` dispatches the selected General/Insomnia/Dream route to either `OpenAiReadyGeneralEndpoint` or `OpenAiCodexGeneralEndpoint`; the latter uses the ChatGPT Codex Responses backend, structured JSON-schema output, SSE streaming, and the route's reasoning effort. For `OpenAiReady`, the dedicated Dream constructor forces exactly one named function/tool call whose parameters are the requested schema and parses only that tool's JSON arguments; ordinary General/Insomnia OpenAI-ready calls retain JSON-schema `response_format`. The OpenAI-ready General transport performs bounded retries for connect/timeouts and retryable HTTP statuses including 429/5xx. OAuth token refresh remains future work.

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
