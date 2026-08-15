# Current Limitations

Parent index: [Documentation index](INDEX.md)

## Purpose
This document owns known incomplete, transitional, or practically limiting behavior in the current rebuild.

## Overview
Purpose-built local configuration, encrypted credential persistence, initial model-switchboard routing/auth attachment, Archive, packed matrices, Archive row bindings, compatibility profiles, vector-generation publication, exact semantic retrieval, and the original default lexical/hybrid retrieval policy are implemented. Production provider transport/login, later semantic databases, and production storage hardening remain incomplete.

## Storage limits
- Chunks are uncompressed and lack container-level checksum/authentication/encryption.
- No object packing, compaction, vacuum, reachability, or reclamation exists; failed vector builders may leave unreachable matrix/binding artifacts.
- No persistent snapshot/checkpoint acceleration exists.
- No concurrent-writer/locking model exists beyond one `Container` file handle.
- Format migration is not implemented; older development CVAs are rejected.
- The physical scanner materializes each chunk transiently, so a very large matrix object can create a large peak allocation even though matrix bytes are not retained in the reopen index.

## Version/history limits
- Archive and Vector Generations are separate mutable semantic timelines with dense local `u64` watermarks and shared CVA-global ordering.
- Packed matrices, Archive-Vector bindings, and Compatibility Profiles are immutable backing objects and do not independently consume semantic versions.
- An unversioned generation payload is inert after a crash.
- The latest generation for each compatibility profile is implicitly current. Explicit retirement, disabling, rollback, and retention policy are not implemented.
- `source_archive_version` is validated against current Archive state and the creation versions of mapped fragments, but a general whole-CVA historical materialization API is not exposed.
- Whole-CVA restore-and-continue/timeline branching remains unresolved now that two mutable semantic domains exist.
- Archive branch/session retention and generation retention/vacuum policies are undefined.

## Compatibility-profile and endpoint limits
- `SimulatedEmbeddingEndpoint` is the only built-in endpoint implementation. No live OpenAI-compatible, provider-native, local-runtime, or other production adapters exist yet.
- Compatibility policy v1 uses fixed probe texts and requires cosine `>= 0.99999` for every corresponding probe. That policy preserves the tolerance lesson from the prior implementation, but it has not yet been calibrated against a representative set of real routed/local embedding endpoints.
- Provider, model, route, and revision are intentionally not compatibility-profile fields. Separate optional provenance metadata has not been designed yet.
- Compatibility profiles currently require dimensions and declared normalization to match exactly. More nuanced compatibility rules, if real endpoints demonstrate a need, remain measurement-driven future work.
- The development generation builder stores endpoint output as `f32` packed rows. Packed storage supports other scalar types, but quantization/dequantization semantics are not yet modeled.
- Low-level `publish_vector_generation` validates references, dimensions, current `f32` representation, and Archive coverage but cannot prove externally supplied vector bytes actually came from the claimed compatibility profile. The high-level builder performs endpoint/profile verification.

## Retrieval limits
- Exact semantic retrieval is implemented as a full cosine scan of one selected compatibility profile's current generation. It is intentionally `O(rows × dimensions)` and has no ANN acceleration yet.
- Each search currently materializes that packed matrix object into memory before scanning it; mapped/segmented/streaming search is not implemented.
- The direct `Cva::semantic_search` API re-runs compatibility probes for every request because no shared long-lived runtime/capability cache exists yet.
- Searchable published generations currently require `f32` matrices. Packed storage still supports other scalar types, but alternate searchable representations wait for explicit quantization/dequantization semantics.
- Lexical retrieval is currently a full fragment-text scan; no persistent lexical index exists yet.
- Default hybrid retrieval uses `30` candidates, `10` final results, and `0.45/0.55` lexical-semantic weights. These defaults can now be overridden through validated `RetrievalConfig`.
- Conversation/time/metadata filters, reranking, and a broader retrieval controller are not implemented in v2 yet.
- Different compatibility profiles have independent current generations, but there is no multi-profile score-fusion policy; scores from unrelated vector spaces must not be compared directly.

## Indexing and memory limits
Archive node/branch/fragment lookup uses dense records plus compact open-addressed slots. Fragment indexes retain one derived `u64` Archive creation version per fragment so generation coverage can be validated. `ContentId -> ChunkRef` remains a direct fixed-width hash table.

Reopen uses one streaming physical pass shared by all concrete stores. Current measurements are recorded in [development](development.md); larger realistic-dimension vector-bearing measurements remain useful before storage optimization.

## Local configuration limits
- Purpose-built `continuity.cfg` persistence is implemented with replaceable logical objects and atomic whole-file replacement.
- The default operating-system config location is not selected yet; callers currently provide the config path.
- Fragment, retrieval, `models.general`, `models.embedding`, and encrypted `credential.<id>` objects are implemented.
- Model routes reference credentials by ID; the switchboard validates auth kind and can attach bearer/auth account headers.
- Direct provider HTTP transport is not implemented yet.
- `openai-codex` device-code acquisition and OAuth token refresh are not implemented yet; stored ChatGPT OAuth tokens can already be attached to requests.
- A 256-bit master key can be generated/reloaded, but it is temporarily stored as plaintext `continuity.master-key.json` beside the config.
- Windows Credential Manager integration is not implemented yet.
- No text import/export format exists yet.

## Product/runtime limits
Memories, Memory Vectors, Graph, the shared long-lived runtime, Insomnia, Dream, Ego, production importers, and durable operational worker state are not implemented in this repository yet.

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

## Notes
These limitations should be updated in the same implementation change that removes them.
