# Current Limitations

Parent index: [Documentation index](INDEX.md)

## Purpose
This document owns known incomplete, transitional, or practically limiting behavior in the current rebuild.

## Overview
Purpose-built local configuration, encrypted credential persistence, model-switchboard routing/auth attachment, direct OpenAI-ready embedding/General transport, provider-native Codex General/Insomnia transport, optional singleton CVA workspace metadata (`id`, `name`, `workspace_type`), Archive, derived conversation/leaf inventory and exact-leaf transcript reads, per-turn source ingestion, a transport-neutral synchronous `InteractionRuntime` with explicit session resume and streamed completed-message assembly, native source attachments, standalone embedded files, filename-only file search, explicit file-to-Memory links, deterministic Episodes, authoritative Memory revisions, Insomnia processing, immutable Memory Vectors, packed matrices, Archive row bindings, compatibility profiles, vector-generation publication, exact semantic retrieval, and the default lexical/hybrid retrieval policy are implemented. Warlock v2 now directly hosts Continuity for CVA workspace lifecycle and durable user-turn conversation start/resume/reopen, but there is still no shared long-lived inference/background host loop, full workspace/CVA management surface, external live interaction adapter, production provider importer suite, Codex OAuth refresh, later Echo/Graph/Dream/Ego layers, general physical compaction/vacuum, or other production storage hardening.

## Storage limits
- Chunks are uncompressed and lack container-level checksum/authentication/encryption.
- `CVACONT1` uses a `u32` content-byte length, so one embedded file/content object must be smaller than 4 GiB; streaming/chunked large-file ingestion is not implemented.
- No general object packing, physical compaction, vacuum, reachability, or reclamation exists; failed vector builders and other future staged-object workflows may still leave unreachable backing chunks. Routine Insomnia queue/claim/renew/retry transitions are runtime-only and add zero CVA bytes; failed extraction or synthesis also adds zero CVA bytes, while successful processing publishes one self-contained atomic completion chunk.
- No persistent snapshot/checkpoint acceleration exists.
- The finite Insomnia drain has a split locking model: Archive is read-only, Memories and Insomnia operational state lock independently, and the single `Container` file cursor/append/version boundary remains serialized. A general long-lived runtime for arbitrary concurrent CVA writers is not implemented yet.
- Format migration is not implemented; older development CVAs are rejected.
- The physical scanner materializes each chunk transiently, so a very large matrix object can create a large peak allocation even though matrix bytes are not retained in the reopen index.

## Version/history limits
- Archive, Memories, and Vector Generations are separate mutable semantic timelines with dense local `u64` watermarks and shared CVA-global ordering. Insomnia queue/lease/retry state is operational and consumes no semantic clock.
- Packed matrices, Memory-Vector bindings, Archive-Vector bindings, and Compatibility Profiles are immutable backing objects and do not independently consume semantic versions. Memory Vectors are keyed by `(CompatibilityProfileId, MemoryBodyId)` and do not have a revision/generation clock.
- An unversioned generation payload is inert after a crash.
- The latest generation for each compatibility profile is implicitly current. Explicit retirement, disabling, rollback, and retention policy are not implemented.
- `source_archive_version` is validated against current Archive state and the creation versions of mapped fragments, but a general whole-CVA historical materialization API is not exposed.
- Whole-CVA restore-and-continue/timeline branching remains unresolved now that three mutable semantic domains exist.
- Archive branch/session retention and generation retention/vacuum policies are undefined.

## Compatibility-profile and endpoint limits
- Direct `openai-ready` embedding/General HTTP execution and provider-native `openai-codex` General/Insomnia Responses execution are implemented. Additional provider-native/local-runtime adapters remain future work.
- Compatibility policy v2 uses the fixed probe suite and requires cosine `>= 0.9998` for every corresponding probe. On 2026-08-15, eight repeated same-route comparisons of `qwen/qwen3-embedding-8b` through OpenRouter produced minimum cosine values from `0.99988147` to `0.99993311`; policy v1's `0.99999` threshold rejected all eight. Broader routed/local endpoint calibration is still incomplete.
- Provider, model, route, and revision are intentionally not compatibility-profile fields. Separate optional provenance metadata has not been designed yet.
- Compatibility profiles currently require dimensions and declared normalization to match exactly. More nuanced compatibility rules, if real endpoints demonstrate a need, remain measurement-driven future work.
- The development generation builder stores endpoint output as `f32` packed rows. Packed storage supports other scalar types, but quantization/dequantization semantics are not yet modeled.
- Low-level `publish_vector_generation` validates references, dimensions, current `f32` representation, and Archive coverage but cannot prove externally supplied vector bytes actually came from the claimed compatibility profile. The high-level builder performs endpoint/profile verification.

## Retrieval limits
- Exact semantic retrieval is implemented as a full cosine scan of one selected compatibility profile's current generation. It is intentionally `O(rows × dimensions)` and has no ANN acceleration yet.
- Each search currently materializes that packed matrix object into memory before scanning it; mapped/segmented/streaming search is not implemented.
- The direct `Cva::semantic_search` API re-runs compatibility probes for every request because no shared long-lived runtime/capability cache exists yet.
- Searchable published generations currently require `f32` matrices. Packed storage still supports other scalar types, but alternate searchable representations wait for explicit quantization/dequantization semantics.
- Normal lexical/hybrid retrieval and Insomnia `archive_search` evidence now use the same disposable in-memory inverted index derived from Archive fragments. Normal queries extend it lazily and incrementally; the finite Insomnia drain prewarms it once so worker threads can rank evidence against immutable index state without the Container lock. The index is not persisted and is rebuilt after reopen. Evidence hydration still reads only the bounded winning turn bodies through the serialized Container handle.
- Default hybrid retrieval uses `30` candidates, `10` final results, and `0.45/0.55` lexical-semantic weights. These defaults can now be overridden through validated `RetrievalConfig`.
- Conversation/time/metadata filters, reranking, and a broader retrieval controller are not implemented in v2 yet.
- Different compatibility profiles have independent current generations, but there is no multi-profile score-fusion policy; scores from unrelated vector spaces must not be compared directly.

## Indexing and memory limits
Archive node/branch/fragment lookup uses dense records plus compact open-addressed slots. Fragment indexes retain one derived `u64` Archive creation version per fragment so generation coverage can be validated. `ContentId -> ChunkRef` remains a direct fixed-width hash table.

Reopen uses one streaming physical pass shared by all concrete stores. Current measurements are recorded in [development](development.md); live 1024-dimensional vectors have been exercised on the prepared 281-fragment corpus, while larger-population realistic-dimension measurements remain useful before storage optimization.

## Local configuration limits
- Purpose-built `continuity.cfg` persistence is implemented with replaceable logical objects and atomic whole-file replacement.
- The default operating-system config location is not selected yet; callers currently provide the config path.
- Fragment, retrieval, `models.general`, optional `models.insomnia`, `models.embedding`, and encrypted `credential.<id>` objects are implemented. Insomnia resolves to `models.general` when its dedicated route is absent.
- Model routes reference credentials by ID; the switchboard validates auth kind and can attach bearer/auth account headers.
- Direct `openai-ready` embedding/General HTTP transport and provider-native `openai-codex` General/Insomnia Responses transport are implemented. Codex routes require explicit reasoning effort and use the stored ChatGPT OAuth credential/account ID.
- `openai-codex` ChatGPT device-code acquisition is implemented through the auth.openai.com device-code protocol. OAuth token refresh is not implemented yet.
- A 256-bit master key can be generated/reloaded, but it is temporarily stored as plaintext `continuity.master-key.json` beside the config.
- Windows Credential Manager integration is not implemented yet.
- No text import/export format exists yet.

## CLI limits
- `cli/` is repo-local and deliberately not installed or built by the core package; invoke it with `cargo run --manifest-path cli/Cargo.toml -- ...`.
- `vectors probe` and `vectors build` use the configured live `openai-ready` embedding route. The `dev` profile/build/search commands remain explicit simulator paths.
- `insomnia run <cva>` uses the configured Insomnia route (falling back to General), registers uncovered canonical import paths unless `--existing-queue-only` is supplied, and invokes the core drain with both General and Embedding endpoints. The core drain automatically fills missing Memory Vectors after authoritative Episode processing.
- Remote embedding defaults are 16 inputs per HTTP request and 16 concurrent requests, based on the 2026-08-15 OpenRouter/Qwen3 throughput benchmark; retry/backoff and provider-specific rate-limit adaptation are not implemented yet.
- `import graph-jsonl` supports one generic development node/branch format and can carry filesystem-backed attachments, but it is still a batch/development importer rather than a production provider importer. No production ChatGPT, Claude, Codex/Hermes, or other provider/session adapters exist yet.
- The CLI has no single-turn live-ingestion command and no file list/add/export/rename/move/provenance-management command surface.

## Product/runtime limits
- Workspace metadata currently supports one-time initialization/read only. Display-name rename, workspace-type migration, resource bindings, cross-workspace links, and host capability registration are not implemented yet.
- Deterministic Archive Episodes, authoritative Memory revision storage, runtime-only transient Insomnia queue/lease/retry transitions, durable final outcomes, and compact successful completion transactions are implemented.
- Insomnia extractor contract `v3-0` uses a clause-level authority/disposition ledger followed by a wording-only synthesis pass. The ledger schema requires every authoritative Episode user turn, owns disposition/authority/lifecycle/provenance/category/type, and may perform one bounded read-only Archive evidence round before finalization. Retained clauses are grouped deterministically; synthesis can return only title/body wording for each required group and cannot mutate provenance/classification or resurrect omitted state. Evidence supports exact historical turns, bounded ancestry ranges, and lexical Archive search; it is capped at four requests and 64 turns / 128 KiB, cannot provide user authority, and external assistant provenance must have been returned by that evidence round. Successful Episode processing publishes zero-or-more new Memory records plus the completion outcome through one atomic completion chunk. A separately validated Luna-low metadata pass improves classification agreement while preserving frozen semantic groups, but that third model call currently exists only in the experimental tuning harness and has not yet been ported into authoritative `v3-0`.
- Archive source records do not currently persist a separate conversation-scope field, so the old implementation's cross-scope evidence rejection cannot yet be reproduced inside one CVA. Evidence is confined to the current CVA; if multiple visibility scopes are later stored in one Archive, scope metadata must be added before cross-scope reads are allowed.
- `create_memory` exists as a library-level tail-finalization + immediate-queue seam; it is not yet exposed through a shared live-model runtime/tool adapter.
- `InteractionRuntime` can synchronously open/resume a session, assemble one in-flight user/agent message from text deltas and complete attachments, durably acknowledge it, attempt size-driven Episode scheduling after completion without coupling that result to acknowledgement, and explicitly finalize inactive session tails. Session/stream buffers are process-local and intentionally non-semantic; an incomplete message is lost on runtime loss and must be replayed by the adapter. Existing durable session history requires an explicit resume message. There is still no automatic adapter reconnect/resume coordinator, continuous inactivity wakeup timer, background Memory/vector loop, IPC/API listener, or adapter lifecycle manager.
- Warlock v2 now directly integrates CVA workspace create/open/close plus durable conversation list/start/resume/user-turn/reopen through Continuity. The broader workspace/CVA application surface remains incomplete: provider/agent execution, attachments in the Warlock composer, branch-selection UI, files, Memories, provenance, selective import/export, and lifecycle management are not yet available through the product surface.
- No live ACP-compatible capture/proxy adapter is implemented yet. ACP is scoped as a first-class external interoperability adapter rather than the primary product interface; Warlock-native interaction and all external adapters are planned to converge through one transport-neutral Continuity interaction seam. ADR 0015 governs ACP capture semantics and ADR 0016 governs the broader product/runtime boundary.
- Automatic 15-minute inactivity scanning is represented by durable timestamp-derived policy methods but no long-lived runtime currently wakes and applies the policy.
- A one-shot whole-backlog worker pool is implemented with 1–64 workers and a default of 48. Workers no longer share one whole-CVA mutex: claim scheduling is indexed, Archive state is read-only during the drain, and Memories/Insomnia operational state have independent locks. Physical Archive reads and durable appends still serialize through the single `Container` file handle. Retryable extraction failures are requeued until the configured attempt limit, while invalid endpoint configuration is terminal. On the 2026-08-15 66-Episode Luna/low corpus sweep, 48 workers was the fastest observed setting and repeated within 1%; bracketing 44/52-worker runs and the 56/64-worker points were slower, so the current maximum remains configurable rather than becoming the default.
- The one-shot worker uses a 15-minute default lease and renews immediately before successful publication; it does not yet run an in-flight heartbeat during one blocking model request. Current OpenAI-ready and Codex General request timeouts are 180 seconds and extraction permits at most two model rounds, but heartbeat/cancellation belongs in the shared long-lived runtime before production.
- Memory Vector storage and automatic missing-body embedding are implemented in the core finite Insomnia drain. A Memory is authoritative before embedding; vectorization/profile errors are returned after Episode processing without undoing Memories, and a later drain retries missing bindings. Continuous background vector work between finite drains still belongs to the future shared long-lived runtime; Echo, Graph, Dream, Ego, production importers, and the native product/management surfaces remain unimplemented.
- Memory authority is independent from embeddings. Memory semantic title/content is immutable across revisions, so metadata-only revisions do not cause embedding regeneration; only a new compatibility profile creates another vector for an existing body.

## Enforcement limits
Repository-local Pitlord policy has not yet been added.

## Related docs
- [Roadmap](roadmap.md)
- [Architecture](architecture.md)
- [Storage format](storage-format.md)
- [Versioning and rollback plan](version-history-plan.md)
- [ADR 0007](decisions/0007-compatibility-profiles-and-vector-generations.md)
- [ADR 0009](decisions/0009-expandable-model-switchboard.md)
- [ADR 0010](decisions/0010-encrypted-credential-objects.md)
- [Repo-local CLI](cli.md)
- [ADR 0011](decisions/0011-detachable-repo-local-cli.md)
- [ADR 0012](decisions/0012-deterministic-episodes-and-insomnia-memory-authority.md)
- [ADR 0013](decisions/0013-immutable-memory-vector-bindings.md)
- [ADR 0015](decisions/0015-acp-inline-interaction-stream.md)
- [ADR 0016](decisions/0016-native-product-surface-and-shared-interaction-runtime.md)

## Notes
These limitations should be updated in the same implementation change that removes them.
