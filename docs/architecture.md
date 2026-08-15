# Architecture
Parent index: [Documentation index](INDEX.md)
## Purpose
This document owns the current Continuity Memory v2 implementation boundaries, state ownership, lifecycle, and code map.
## Overview
A `.cva` is one physical file containing explicit concrete owners:
```text
Cva
├── Container
│   └── global_version: u64
├── Archive
│   ├── archive_version: u64
│   ├── conversation/session-local ancestry
│   └── immutable Episodes
├── Memories
│   └── memory_version: u64 + immutable revisions
├── InsomniaOperational
│   └── queue / priority / leases / retries / attempts
├── PackedVectorStore
│   └── immutable numeric matrices
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
ContinuityConfig
└── continuity.cfg
    ├── archive.fragments
    ├── retrieval.default
    ├── models.general
    ├── models.embedding
    └── credential.<id> (AES-256-GCM)
```
`continuity.cfg` is current-state configuration only. It is not contained in a `.cva`, consumes no semantic clocks, and has no append-only/history semantics.
## Responsibilities
### Local configuration
`ContinuityConfig` owns one purpose-built replaceable config file. Logical objects have stable keys and typed payload schemas; replacing a setting rewrites one complete current file image through a temporary-file + atomic-replace lifecycle. Unknown objects are preserved so the object vocabulary can expand. Ordinary objects are unencrypted; credential objects are authenticated encrypted payloads.
### Repo-local CLI
`cli/` is a separate, non-installed Cargo package that depends only on the public library API. It owns argument parsing, secret prompting, and human-readable command composition; it owns no CVA/config/auth/retrieval semantics and can be removed or detached without changing the core package.
### Model switchboard
`ModelSwitchboardConfig` owns machine-local endpoint selection for explicit model capabilities. The initial capabilities are `General` and `Embedding`; the initial providers are `OpenAiCodex` and `OpenAiReady`. Each configured route carries a stable `CredentialId`. `OpenAiCodex` currently supports only General and uses provider-owned routing; `OpenAiReady` supports General and Embedding and requires an explicit HTTP(S) endpoint URL.

`CredentialsConfig` owns decrypted in-memory credentials loaded from encrypted `credential.<id>` config objects. Constructing `ModelSwitchboard` validates that every selected route resolves to a credential of the provider's required auth kind. The switchboard can then produce request auth: bearer API key for `OpenAiReady`, or bearer ChatGPT access token plus optional `ChatGPT-Account-ID` for `OpenAiCodex`. `OpenAiReadyEmbeddingEndpoint` consumes the validated embedding route/auth, performs direct OpenAI-compatible HTTP embedding requests, preserves input order across concurrent batches, and normalizes output according to the declared route contract. `OpenAiReadyGeneralEndpoint` consumes the General route/auth and performs strict JSON-schema chat completions for subsystems such as Insomnia. Provider/model/URL/credential choices remain routing policy and do not enter Compatibility Profile identity or decide vector compatibility.
### Master key
`MasterKeyStore` owns retrieval/creation of the 256-bit key that protects credential objects. The current `JsonMasterKeyStore` is a temporary implementation that stores plaintext `continuity.master-key.json` beside `continuity.cfg`; production storage will move behind the same ownership boundary to the operating-system credential store. Credential ciphertext uses AES-256-GCM with fresh nonces and object-key-bound authenticated data. The master key is machine-local and is never CVA semantic state.
### Container
Container owns the fixed header, opaque length-prefixed chunks, `ChunkRef`, file I/O, sync, the single physical reopen scan, and CVA-global monotonic version tickets. Global version is ordering only.
### Archive
Archive owns source-history semantics: content-addressed text, immutable conversation nodes, conversation-local parent ancestry, branch/session-head revisions, fragments, immutable deterministic Episodes, the dense Archive watermark, historical branch lookup, and Archive-owned derived indexes.
`archive_version` is a whole-Archive mutation cut. It is not conversation ancestry.

Episodes are contiguous ancestry ranges made from whole user-led response cycles. They are finalized by size, 15-minute configurable inactivity, finite-import end, or the narrow `create_memory` request. Finalizing an Episode never closes its conversation. No semantic topic detector participates in Episode identity.

### Memories
Memories owns authoritative working-memory revisions. One stable `MemoryId` has immutable numbered revisions; publication requires the expected current revision and a stable mutation ID for idempotent replay. Title/content bodies are content-addressed separately from revision metadata. Each published revision advances dense `memory_version` and consumes one CVA-global ordering ticket. Archive/Episode provenance is validated at write and reopen. Memory authority does not depend on vector availability.

### Insomnia operational state
Insomnia operational state owns finalized-Episode processing coordination rather than another semantic timeline. Every finalized Episode is work. Priority is immediate live (`create_memory`), normal live, then import/backfill, with oldest source chronology inside each class. Queue registration is idempotent. Claims use expiring lease tokens; stale tokens cannot finalize reclaimed work. Retry and terminal outcomes plus immutable attempt history are retained. Processing claims are reclaimable after reopen. This owner consumes no semantic version clock.

`create_memory` is not a Memory write API. It finalizes the current uncovered live Episode tail and queues it at immediate-live priority; Insomnia remains the only authority that can turn source material into working-memory revisions.

### PackedVectorStore
Packed vectors own immutable matrix bytes and physical row representation. `VectorSchema` defines dimensions and scalar representation; rows are fixed-width and contiguous. Equal schema+bytes deduplicate.
Packed vectors do not know which Archive records rows represent or which embedding space produced them.
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
        ├── Insomnia operational state
        ├── PackedVectorStore
        ├── ArchiveVectorStore
        ├── CompatibilityProfileStore
        └── VectorGenerationStore
```
After the scan, cross-store references are validated in dependency order. Full packed matrices and Archive-Vector mappings are not retained in steady-state indexes.
## Ordering model
Archive, Memories, and Vector Generations are independently mutable semantic domains:
```text
G100 / A700   Archive mutation
G101 / M12    Memory revision
G102 / V20    vector generation
G103 / A701   Archive mutation
```
`G`, `A`, `M`, and `V` are ordering/watermark integers, not parent relationships. Conversation ancestry remains node-local. Compatibility profiles, vector backing objects, and Insomnia operational coordination are not semantic timeline events.
## Invariants and safety boundaries
- one physical CVA owner and one shared reopen scan;
- no generalized semantic database/root/dependency layer;
- Archive and vector-generation local clocks remain independent;
- each global version is claimed by at most one semantic mutation;
- packed matrices, Archive-Vector bindings, and compatibility profiles are immutable backing objects;
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
| physical Container/global clock | `src/container*.rs` |
| Archive/history/fragments/Episodes | `src/archive*.rs`, `src/fragment*.rs`, `src/episode*.rs` |
| Memories | `src/memory*.rs` |
| Insomnia extraction/processing/operational scheduling | `src/insomnia.rs`, `src/insomnia/**/*.rs` |
| packed matrices | `src/packed_vector_*.rs` |
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
## Notes
Codex/provider-native General transport, search filters, reranking, ANN acceleration, explicit generation retirement, and whole-CVA restore-and-continue remain separate slices.
