# Architecture
Parent index: [Documentation index](INDEX.md)
## Purpose
This document owns the current Reliquary Memory v2 implementation boundaries, state ownership, lifecycle, and code map.
## Overview
A `.cva` is one physical file containing explicit concrete owners:
```text
Cva
├── WorkspaceMetadata
│   ├── stable workspace ID
│   ├── display name
│   └── workspace type
├── Container
│   └── global_version: u64
├── Archive
│   ├── archive_version: u64
│   ├── conversation/session-local ancestry
│   ├── source turns + native attachments + embedded files
│   ├── explicit file-to-Memory links
│   └── immutable Episodes
├── Memories
│   └── memory_version: u64 + immutable revisions
├── Graph
│   ├── graph_version: u64 + append-only relationship mutations
│   ├── persistent MemoryId ↔ dense NodeId catalogue
│   └── active topology/traversal via arcana-graph
├── InsomniaOperational
│   └── queue / priority / leases / retries / compact completion receipt
├── PackedVectorStore
│   └── immutable numeric matrices
├── MemoryVectorStore
│   └── immutable (profile, MemoryBodyId) -> packed row bindings
├── ArchiveVectorStore
│   └── immutable row -> FragmentId bindings
├── CompatibilityProfileStore
│   └── immutable vector-space compatibility contracts
└── VectorGenerationStore
    └── vector_version: u64 + current generation per profile
```
`Cva` owns physical composition and the single Container handle. It is not a generalized semantic database, store registry, root, or dependency engine.
Machine-local application configuration is a separate owner:
```text
ReliquaryConfig
└── reliquary.cfg
    ├── archive.fragments
    ├── retrieval.default
    ├── models.general
    ├── models.insomnia (optional; falls back to general)
    ├── models.dream (optional; falls back to general)
    ├── models.embedding
    └── credential.<id> (AES-256-GCM)
```
`reliquary.cfg` is current-state configuration only. It is not contained in a `.cva`, consumes no semantic clocks, and has no append-only/history semantics.
## Responsibilities
### Workspace metadata
`WorkspaceMetadataStore` owns one optional singleton workspace identity inside the CVA: stable ID, display name, and extensible workspace-type identifier. `Cva::create_workspace` initializes the record at creation; an ordinary CVA may be initialized exactly once later. Current workspace metadata is clock-neutral and does not participate in Archive, Memory, Vector Generation, or CVA-global semantic ordering. The owner is deliberately not a generic metadata/property store. Rename/type-change history and external-resource bindings are not part of this first slice.

### CVA comparison and reconciliation
`Cva::compare` requires stable workspace identity on both files, rejects mismatched workspace IDs, and compares physical chunk streams to find their longest common prefix. `Cva::reconcile` builds on that detection seam: identical/strict-extension cases copy the complete valid side, while true divergence semantically replays the complete left history plus the right divergent tail. If the right/candidate side contributes new semantic state, the result is a fresh workspace CVA with reallocated global/Archive/Memory/Graph clocks; if every candidate record is already semantically present, the fresh probe is discarded and an exact copy of the canonical/left CVA is returned instead. This no-op path prevents repeated cloud-conflicted copies from rewriting the workspace and avoids any reconciliation-receipt metadata inside the CVA. Replay is owner-ordered: Archive source/Files/Branches/Episodes/Fragments first, Memory revisions next, then Graph transactions after their Memory endpoints exist, followed by cross-owner file-to-Memory links and durable Insomnia receipts; interaction-stream state and compatibility profiles are also preserved through their own seams. Graph single-edge and atomic batch mutations are republished through the Graph owner with fresh destination clocks and dense node mappings. For each oriented `(source, target, kind)` identity, divergent active-state histories are prefix-compared so already-absorbed/stale mutations are skipped and only a novel right suffix is replayed; non-prefix histories fail closed. Graph topology and Dream's duplicate index remain derived and rebuild from merged relationship authority. Memory mutation/revision invariants and completion identity similarly decide duplicates versus genuine conflicts. Immutable Fragments are revalidated and retained, and the disposable lexical index rebuilds lazily from a changed merged state. Packed vectors, Memory/Archive vector bindings, and Vector Generations are retired only for a real changed merge; `canonical_change_required` and `vector_rebuild_required` report those outcomes separately. Semantic replay collisions are normalized into public `CvaReconcileConflict` values carrying the stable owner identity and the competing state needed by a host UI; corruption and transport failures remain ordinary errors. `Cva::reconcile_and_promote` skips replacement entirely for semantic no-ops and otherwise uses a hidden sibling candidate, pre-promotion fingerprint check, synced recovery copy, atomic path replacement, post-replacement reopen/sync, recovery on finalization failure, and cleanup. It detects ordinary canonical changes during preparation but does not introduce a general interprocess writer lock or cloud-provider discovery. Cloud providers remain responsible for storage transport and conflicted-copy preservation. See [ADR 0019](decisions/0019-cloud-backed-cva-reconciliation.md).

### Local configuration
`ReliquaryConfig` owns one purpose-built replaceable config file. Logical objects have stable keys and typed payload schemas; replacing a setting rewrites one complete current file image through a temporary-file + atomic-replace lifecycle. Unknown objects are preserved so the object vocabulary can expand. Ordinary objects are unencrypted; credential objects are authenticated encrypted payloads.
### Repo-local CLI
`cli/` is a separate, non-installed Cargo package that depends only on the public library API. It owns argument parsing, secret prompting, and human-readable command composition; it owns no CVA/config/auth/retrieval semantics and can be removed or detached without changing the core package.
### Model switchboard
`ModelSwitchboardConfig` owns machine-local endpoint selection for explicit model capabilities. The current capabilities are `General`, `Insomnia`, `Dream`, and `Embedding`; the providers are `OpenAiCodex` and `OpenAiReady`. Insomnia and Dream may each have dedicated routes and otherwise resolve to General. Each configured route carries a stable `CredentialId`. `OpenAiCodex` supports General/Insomnia/Dream routing and uses provider-owned routing; `OpenAiReady` supports General/Insomnia/Dream/Embedding and requires an explicit HTTP(S) endpoint URL.

`CredentialsConfig` owns decrypted in-memory credentials loaded from encrypted `credential.<id>` config objects. Constructing `ModelSwitchboard` validates that every selected route resolves to a credential of the provider's required auth kind. The switchboard can then produce request auth: bearer API key for `OpenAiReady`, or bearer ChatGPT access token plus optional `ChatGPT-Account-ID` for `OpenAiCodex`. `OpenAiReadyEmbeddingEndpoint` consumes the validated embedding route/auth, performs direct OpenAI-compatible HTTP embedding requests, preserves input order across concurrent batches, and normalizes output according to the declared route contract. `OpenAiReadyGeneralEndpoint` consumes the General route/auth and performs strict JSON-schema chat completions for subsystems such as Insomnia. Provider/model/URL/credential choices remain routing policy and do not enter Compatibility Profile identity or decide vector compatibility.
### Master key
`MasterKeyStore` owns retrieval/creation of the 256-bit key that protects credential objects. The current `JsonMasterKeyStore` is a temporary implementation that stores plaintext `reliquary.master-key.json` beside `reliquary.cfg`; production storage will move behind the same ownership boundary to the operating-system credential store. Credential ciphertext uses AES-256-GCM with fresh nonces and object-key-bound authenticated data. The master key is machine-local and is never CVA semantic state.
### Container
Container owns the fixed header, opaque length-prefixed chunks, `ChunkRef`, file I/O, sync, the single physical reopen scan, and CVA-global monotonic version tickets. Global version is ordering only.
### Archive
Archive owns source-history semantics: content-addressed content bytes, immutable conversation nodes, native source-turn attachments, standalone embedded file manifests, conversation-local parent ancestry, branch/session-head revisions, derived conversation/leaf inventory and exact-leaf transcript resolution, fragments, immutable deterministic Episodes, explicit file-to-Memory links, the dense Archive watermark, historical branch lookup, and Archive-owned derived indexes. A source turn with attachments enters through one `IncomingTurn` ingestion boundary: body and attachment bytes may be staged as content-addressed backing objects, but the node, attached file manifests, and source provenance become semantically visible together under one Archive publication. Source attachment provenance is therefore part of the source event, not a generic association operation. Later file-to-Memory relationships remain explicit stable-ID links and do not transfer file ownership to Memories.
`archive_version` is a whole-Archive mutation cut. It is not conversation ancestry.

Conversation inventory is derived from current Archive nodes: every durable leaf is exposed, and transcript resolution requires the caller to supply the exact leaf node ID. This adds no persistent conversation-head record and does not silently select among branches.

The current source-ingestion execution boundary has three library-level seams. `Cva::ingest_turn` is the Archive-facing primitive. `InteractionRuntime` is the transport-neutral synchronous session/message seam. `ReliquaryRuntimeHost` is the in-process long-lived service owner used by Warlock: it wraps the interaction runtime, sleeps/wakes Insomnia workers, applies inactivity finalization, and owns a dedicated Memory Vector backfill worker. Session identity maps to Archive conversation identity, message identity maps to node identity, the current durable session leaf becomes the next message parent, normalized `User`/`Agent` roles map to Archive `user`/`assistant`, and source success is returned only after `Cva::sync()` completes. Model and embedding calls run outside the shared interaction-runtime lock; claiming/snapshotting, bounded evidence hydration, Memory publication, and vector commits acquire it only for their local CVA operations. General and embedding endpoints may be attached or replaced while the host remains alive. Warlock owns the host lifetime for one CVA workspace; Reliquary owns these background service mechanics. External adapter reconnect/resume coordination and broader file/Memory/provenance management surfaces remain future work. The host is also the intended lifecycle attachment point for Dream and Ego; see [Current limitations](current-limitations.md), [Roadmap](roadmap.md), and [ADR 0017](decisions/0017-cva-workspace-and-warlock-host-application.md).

Episodes are contiguous ancestry ranges made from whole user-led response cycles. They are finalized by size, 15-minute configurable inactivity, finite-import end, or the narrow `create_memory` request. Finalizing an Episode never closes its conversation. No semantic topic detector participates in Episode identity.

### Memories
Memories owns authoritative working-memory revisions. One stable `MemoryId` has immutable numbered revisions; publication requires the expected current revision and a stable mutation ID for idempotent replay. Title/content bodies are content-addressed separately from revision metadata. A Memory's `MemoryBodyId` is immutable across revisions: metadata/classification/lifecycle may change, but semantic title/content cannot mutate in place. Semantic corrections create another Memory rather than rewriting an existing body. Each published revision advances dense `memory_version` and consumes one CVA-global ordering ticket. Archive/Episode provenance is validated at write and reopen. Insomnia persists assistant-authored **authority provenance** in the existing `content_source_*` record fields only when the user adopts/retains that assistant proposition; separate `grounding_source_*` fields identify context used only to resolve a referent in a user-owned proposition. Grounding never supplies semantic authority. Memory authority does not depend on vector availability.

### Graph
Graph owns durable Memory-to-Memory semantic relationships. Stable `MemoryId` endpoints are mapped once to dense `arcana_graph::NodeId` values for topology operations; those dense IDs are an internal index and never replace Memory identity. Relationship mutations are oriented records with one of `topical`, `factual`, `causal`, `recurrent`, `references`, `duplicate-of`, `supersedes`, or `structural-parent`; one non-empty single-edge or multi-edge transaction advances dense `graph_version` once and consumes one CVA-global ordering ticket. Retraction records the same oriented relationship identity with `active=false` rather than deleting history. Processing or scheduling order does not determine edge direction.

Reliquary owns Graph durability, versioning, Memory endpoint validation, and reopen reconstruction. The pinned `arcana-graph` crate supplies repository-agnostic graph primitives, adjacency/topology structures, and traversal algorithms. Arcana repository identities, relation vocabulary, repository snapshots, and protocol semantics are not part of the CVA Graph owner. Pre-Graph CVAs reopen with an empty Graph owner and acquire the Graph format marker lazily on the first relationship write.

### Insomnia operational state
Insomnia operational state owns finalized-Episode processing coordination rather than another semantic timeline. Every finalized Episode is work. Priority is immediate live (`create_memory`), normal live, then import/backfill, with oldest source chronology inside each class. Queue registration is idempotent. Claims use expiring lease tokens; stale tokens cannot finalize reclaimed work. Pending, processing, lease-renewal, and retryable-failure transitions are runtime-only and are re-derived as Pending after reopen when no final outcome exists. A Terminal outcome remains durable as one final work record; a successful outcome atomically publishes all newly created Memory records together with one compact Episode-completion transaction. Prior failed-attempt history is not retained as active history after success. This owner consumes no semantic version clock.

`create_memory` is not a Memory write API. It finalizes the current uncovered live Episode tail and queues it at immediate-live priority; Insomnia remains the only authority that can turn source material into working-memory revisions.

Insomnia extractor contract `v3-0` is two-pass. Pass 1 emits a clause-level authority/disposition ledger with a schema that requires every authoritative Episode user turn to be accounted for and constrains assistant-authority/grounding IDs to available Episode or bounded-evidence turns. Pass 1 owns retain/omit/supersede disposition, authority kind, lifecycle, source identity, assistant-authority identity, grounding identity, category, and type. Retained clauses are then deterministically grouped only when those structural fields match. Pass 2 receives fixed groups and may return only one title/body wording pair for each required group key; it cannot add, drop, merge, split, reclassify, or alter provenance. Reliquary reconstructs exact provenance bytes from the selected source IDs and includes deterministic ledger semantic material in candidate identity so structurally distinct Memories from one user turn remain distinct even when they share full-turn provenance.

The one-shot backlog worker is the current concurrency boundary for Insomnia. `Cva::drain_insomnia_backlog` accepts 1–64 workers (default 48, calibrated against the current Luna/low corpus run). During a drain, Archive semantic indexes are shared read-only while Container, Memories, and Insomnia operational state have separate locks. General-model inference holds no CVA lock. Archive body reads and durable state publication take the Container lock only for the single-file cursor/append/version boundary; Memory and Insomnia mutations additionally take only their owning store lock in a fixed Container → MemoryStore → InsomniaStore order. Insomnia claim selection uses an incrementally maintained ordered ready/retry/lease index rather than scanning lifetime work history. Failed extraction attempts retain source-priority ordering when they become retry-eligible; invalid endpoint configuration is terminal, and the configured maximum attempt count prevents a persistent provider/model failure from spinning indefinitely. The drain returns only after no pending, processing, or retryable-failed work remains.

For whole-file import bring-up, `finalize_canonical_imports_and_queue` materializes uncovered deterministic Episodes on canonical Archive branches and idempotently registers all import-origin Episodes as Insomnia work. The concurrent backlog drain itself owns automatic Memory-Vector completion: after authoritative Episode processing finishes, it establishes/reuses the supplied embedding compatibility profile and fills only missing `(profile, MemoryBodyId)` bindings. Embedding failure cannot roll back a published Memory; a later drain retries missing bindings even when no Episode work remains. The repo-local `insomnia run` command supplies the configured embedding endpoint to this core path rather than performing a separate vectorization phase.

Insomnia extraction may perform one bounded read-only Archive evidence round when an explicit callback, adopted/retained assistant proposition, or genuinely unresolved referent cannot be resolved from the authoritative Episode and nearby ancestry. The model may request at most four reads: an exact `(conversation_id, node_id)` turn, a maximum-64-node ancestry range, or lexical Archive search with at most five fragment hits. Returned evidence is deduplicated and capped globally at 64 turns / 128 KiB. Historical evidence can supply an assistant **authority source** only when the authoritative user turn explicitly adopts/retains that proposition, or a user/assistant **grounding source** used only to identify a referent. Neither form of evidence can provide user authority: `source_node_id` remains a user turn inside the authoritative Episode. External provenance is accepted only when that exact historical turn was actually returned in the bounded evidence set, and it may not postdate the user authority turn. A second evidence round is rejected.

### PackedVectorStore
Packed vectors own immutable matrix bytes and physical row representation. `VectorSchema` defines dimensions and scalar representation; rows are fixed-width and contiguous. Equal schema+bytes deduplicate.
Packed vectors do not know which Archive fragments or Memory bodies rows represent or which embedding space produced them.
### MemoryVectorStore
Memory Vectors are immutable derived bindings over shared `PackedVectorStore` matrices. Their durable identity is `(CompatibilityProfileId, MemoryBodyId)`, not Memory revision. Each profile/body pair may bind to exactly one packed row; metadata-only Memory revisions therefore require no vector work. A genuinely new compatibility profile may add another immutable vector for the same Memory body. Memory Vectors consume no semantic/global version clock and have no update/regeneration path.

The high-level `build_missing_memory_vectors` path verifies the endpoint against the selected profile, finds current Memory bodies without a binding for that profile, embeds only those bodies in Document mode, appends one `f32` packed matrix, and records the row bindings. Re-running it after metadata-only revisions creates nothing.

### Dream candidate retrieval

The first Dream implementation seam is read-only bounded Memory candidate discovery. `Cva::dream_candidates` takes an existing source Memory plus one Compatibility Profile and reuses the source Memory's already-stored Document vector; candidate discovery performs no new embedding or model call.

The current lanes are exact cosine similarity over current Memory-body vectors, a bounded prior-semantic quota keyed to authoritative source chronology, and deterministic lexical/metadata overlap. Archived Memories are excluded. Candidates missing a vector under the selected profile may still enter through the lexical/metadata lane. Lane ranks are fused deterministically, with stable `MemoryId` tie-breaking.

The source and every returned candidate are materialized as `DreamMemoryContext`: current Memory metadata/body identity, authoritative source timestamp when provenance resolves one, and all active Graph relationships touching that Memory. Source chronology resolves adopted assistant content provenance first, then the exact user-authority turn through Episode provenance, with the Episode source boundary only as a fallback. `Memory.created_at_ns` and grounding-only provenance do not define semantic source time.

Candidate retrieval owns no durable Dream state, publishes no Graph relationship, and changes no Memory lifecycle.

### Dream pair classification

`DreamClassifier` is the implemented Milestone B inference seam. It consumes `DreamMemoryContext` pairs or an entire bounded `DreamCandidateSet` and calls a strict structured-output `GeneralEndpoint`. Before the model call, each pair is canonicalized by stable `MemoryId` into A/B order; invoking the same pair from the opposite processing endpoint therefore produces the same model payload and cannot redefine semantic direction.

The current classifier proposes exactly one primary relation per evaluation from `none`, `topical`, `factual`, `causal`, `recurrent`, `duplicate_of`, or `supersedes`. `topical`, `recurrent`, and `duplicate_of` require `undirected`; `factual`, `causal`, and `supersedes` require `a_to_b` or `b_to_a`; `none` requires direction `none`. This one-proposal classifier contract does not yet settle whether persistent Graph state may later carry additional orthogonal relationship kinds for the same pair.

Every non-`none` proposal must contain exactly one short verbatim quote from each Memory. Continuity rejects relation/direction mismatches, duplicate/missing evidence sides, and evidence that is not literal Memory title/content text. Source timestamps are supplied as chronology context but are explicitly not proof of causality or supersession; attached Graph context is supplemental, not proof. Classification is transient and read-only: there is still no verifier, accepted-relationship publication, duplicate-chain mutation, or lifecycle side effect.

### ArchiveVectorStore
Archive Vectors own one relationship only:
```text
ArchiveVectorSet
├── packed_vector_id
└── ordered FragmentIds
    row N -> fragment_ids[N]
```
Creation/reopen require an existing matrix, exact row count, real unique Archive fragments, and valid content identity. The persistent object contains no compatibility profile or Archive watermark.
The derived Archive-Vector index records the newest Archive creation version among mapped fragments. That value is not persistent identity; it supports generation coverage validation.
### CompatibilityProfileStore
A compatibility profile is an endpoint-independent contract for one usable vector space:
```text
CompatibilityProfile
├── dimensions
├── normalization
├── probe-suite version
├── compatibility-policy version
└── 4 reference vectors
    ├── 2 Query probes
    └── 2 Document probes
```
Provider, model name, route, and revision are not profile fields. They may be provenance elsewhere later, but they do not prove vector compatibility.
`CompatibilityProfileId` content-addresses the exact stored contract artifact. Compatibility itself is checked separately by embedding the fixed probes and comparing corresponding vectors with cosine similarity. Policy v2 requires every probe to reach `>= 0.9998` and requires dimensions, normalization, probe-suite version, and policy version to match. The threshold was calibrated against live same-route drift measured through OpenRouter rather than exact deterministic simulator output.
Establishing a profile probes the endpoint once. If the resulting candidate is tolerantly compatible with an existing profile, the existing profile is reused even when its exact returned floats differ slightly. Otherwise a new profile is stored. Profile creation is clock-neutral.
### VectorGenerationStore
Vector Generations own active vector-population publication:
```text
VectorGeneration
├── compatibility_profile_id
├── archive_vector_id
├── source_archive_version
├── global_version
└── vector_version
```
`vector_version` is a dense local watermark for generation publications. The newest generation for each compatibility profile is current; older generations remain retained and can be resolved at a historical vector-version cut.
Generation publication is the first vector-layer operation that consumes a CVA-global ticket. Source Archive versions cannot regress for a profile, exceed current Archive state, or predate any mapped fragment. Until quantization semantics are defined, published generations must reference `f32` matrices.
### Retrieval
Retrieval is a read-only query layer, not another database. `Cva::semantic_search` is the low-level semantic channel: it verifies the supplied endpoint against the selected compatibility profile, embeds the raw query in Query mode, resolves that profile's current Vector Generation, exact-scans its packed matrix by cosine, drops scores `<= 0`, and maps row ordinals back to Archive fragments.
`Cva::search` restores the original default product behavior. It independently takes up to 30 lexical and 30 semantic candidates, merges by `FragmentId`, preserves a single available channel's score undiluted, otherwise combines `0.45 × lexical + 0.55 × semantic`, sorts, removes duplicate conversation/ranges, greedily penalizes overlapping fragments from conversations already selected, retains a 30-candidate diversified pool, and returns the first 10. Lexical scoring is the original `0.85 × query-term coverage + 0.15 × bounded term-frequency density` formula.

The current direct `Cva` API performs compatibility verification for each semantic search call because no shared long-lived runtime/capability cache exists yet. Retrieval allocates no semantic versions and persists no retrieval state.
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
### Compatibility-profile establishment
```text
embed fixed Query + Document probes
    ↓
validate dimensions/normalization
    ↓
compare tolerantly with existing profiles
    ├── compatible -> reuse existing profile
    └── none compatible -> store immutable profile
```
No semantic clock advances.
### Vector generation publication
```text
compatibility profile + packed matrix + ArchiveVectorSet exist
    ↓
append immutable generation payload
    ↓
allocate global version G
    ↓
allocate vector version V
    ↓
append generation metadata { G, V, record }
```
An unversioned generation payload is inert.
### Development generation builder
`build_archive_vector_generation` verifies the supplied endpoint against the selected compatibility profile, snapshots the current Archive watermark, embeds all durable fragments in deterministic `FragmentId` order using Document mode, writes an `f32` packed matrix and Archive-Vector binding, then publishes the generation.
## Reopen
```text
one physical chunk scan
    ├── Container framing/global-ticket validation
    └── Cva dispatches each payload
        ├── Archive (including Episodes)
        ├── Memories
        ├── Graph
        ├── Insomnia operational state
        ├── PackedVectorStore
        ├── MemoryVectorStore
        ├── ArchiveVectorStore
        ├── CompatibilityProfileStore
        └── VectorGenerationStore
```
After the scan, cross-store references are validated in dependency order. Full packed matrices and Archive-Vector mappings are not retained in steady-state indexes.
## Ordering model
Archive, Memories, Graph, and Vector Generations are independently mutable semantic domains:
```text
G100 / A700   Archive mutation
G101 / M12    Memory revision
G102 / R8     Graph relationship mutation
G103 / V20    vector generation
G104 / A701   Archive mutation
```
`G`, `A`, `M`, `R`, and `V` are ordering/watermark integers, not parent relationships. Conversation ancestry remains node-local. Compatibility profiles, vector backing objects, and Insomnia operational coordination are not semantic timeline events.
## Invariants and safety boundaries
- one physical CVA owner and one shared reopen scan;
- no generalized semantic database/root/dependency layer;
- Archive, Memory, Graph, and vector-generation local clocks remain independent;
- each global version is claimed by at most one semantic mutation;
- packed matrices, Memory-Vector bindings, Archive-Vector bindings, and compatibility profiles are immutable backing objects;
- Memory vectors bind immutable `MemoryBodyId` content per compatibility profile; Memory revision metadata cannot invalidate or refresh them;
- Archive Vectors own row-to-fragment identity only;
- compatibility profiles own vector-space compatibility contracts, not endpoint provenance;
- endpoint compatibility is tolerant probe comparison, never provider/model labels or exact probe hashes;
- generations own profile-to-ArchiveVector association, coverage, activation, and vector semantic ordering;
- generation matrix dimensions must match the compatibility profile;
- published generations currently require `f32` matrices until alternate representation semantics exist;
- generation source watermark must cover every mapped fragment;
- the semantic retrieval channel searches exactly one selected profile's current generation and never mixes profile score spaces;
- default hybrid retrieval preserves the original 30-candidate / 10-result policy and `0.45/0.55` lexical-semantic fusion;
- local configuration is current-state machine configuration, not CVA semantic state or an internal history system;
- credential objects are encrypted independently and never become CVA semantic state;
- model routes reference credentials by stable ID; executable switchboards reject missing or wrong-kind credentials;
- model-switchboard routing/authentication is machine-local integration policy and cannot establish vector compatibility.
## Code map
| Responsibility | Primary code |
| --- | --- |
| local configuration | `src/config*.rs` |
| detachable operator CLI | `cli/src/*.rs` |
| encrypted credentials | `src/credential*.rs`, `src/config_credentials.rs` |
| model switchboard/provider capabilities/auth binding | `src/model_switchboard*.rs`, `src/model_auth.rs` |
| master key / temporary key store | `src/master_key*.rs` |
| CVA composition/lifecycle | `src/cva.rs`, `src/cva_lifecycle.rs`, `src/cva_*` |
| workspace identity/type | `src/workspace_metadata*.rs`, `src/cva_workspace.rs` |
| physical Container/global clock | `src/container*.rs` |
| Archive/history/fragments/Episodes | `src/archive*.rs`, `src/fragment*.rs`, `src/episode*.rs` |
| native turn/file ingestion | `src/turn_ingest_*.rs`, `src/source_attachment_index.rs`, `src/file*.rs`, `src/cva_turn_ingest.rs`, `src/cva_file_memory.rs` |
| normalized interaction/runtime seam | `src/interaction_model.rs`, `src/interaction_error.rs`, `src/interaction_runtime.rs`, `src/interaction_session.rs`, `src/interaction_stream.rs` |
| Memories | `src/memory*.rs` |
| Insomnia extraction/processing/operational scheduling | `src/insomnia.rs`, `src/insomnia/**/*.rs` |
| packed matrices | `src/packed_vector_*.rs` |
| Memory body/profile row bindings | `src/memory_vector_*.rs`, `src/cva_memory_vectors.rs` |
| Archive row bindings | `src/archive_vector_*.rs` |
| embedding execution/compatibility | `src/embedding_endpoint.rs`, `src/openai_ready_embedding*.rs`, `src/compatibility_profile_*.rs` |
| generation publication/history | `src/vector_generation_*.rs` |
| exact semantic retrieval | `src/semantic_search*.rs` |
| lexical + hybrid retrieval | `src/lexical_search.rs`, `src/search*.rs` |
| corpus vector/retrieval smoke | `examples/vector_generation_smoke.rs` |
## Related docs
- [Storage format](storage-format.md)
- [Local configuration](configuration.md)
- [Repo-local CLI](cli.md)
- [Rust API](api.md)
- [Architectural invariants](invariants.md)
- [Versioning and rollback plan](version-history-plan.md)
- [ADR 0011](decisions/0011-detachable-repo-local-cli.md)
- [ADR 0013](decisions/0013-immutable-memory-vector-bindings.md)
- [ADR 0016](decisions/0016-native-product-surface-and-shared-interaction-runtime.md)
- [ADR 0017](decisions/0017-cva-workspace-and-warlock-host-application.md)
## Notes
Unimplemented behavior is tracked in [Current limitations](current-limitations.md); future implementation sequencing is tracked only in [Roadmap](roadmap.md).
