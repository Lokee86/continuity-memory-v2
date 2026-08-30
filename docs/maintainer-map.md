# Reliquary Memory v2 Maintainer Map

Parent index: [Documentation index](INDEX.md)

## Purpose

This map routes common change intents to canonical documentation, implementation boundaries, and verification surfaces.

## Overview

Use this when ownership is unclear. It does not replace focused architecture/reference documentation.

## Change-area routing

| Change area | Canonical documentation | Primary implementation boundary | Verification |
| --- | --- | --- | --- |
| Repo-local CLI / operator commands | [Repo-local CLI](cli.md), [ADR 0011](decisions/0011-detachable-repo-local-cli.md) | `cli/src/*.rs` | separate CLI fmt/check/test + command smokes |
| Local configuration | [Local configuration](configuration.md), [ADR 0008](decisions/0008-purpose-built-local-configuration.md) | `src/config*.rs` | `src/config_tests.rs` |
| Encrypted credentials | [Local configuration](configuration.md), [ADR 0010](decisions/0010-encrypted-credential-objects.md) | `src/credential*.rs`, `src/config_credentials.rs` | `src/credential_tests.rs` |
| Model switchboard / provider auth | [Architecture](architecture.md), [Local configuration](configuration.md), [ADR 0009](decisions/0009-expandable-model-switchboard.md), [ADR 0010](decisions/0010-encrypted-credential-objects.md) | `src/model_switchboard*.rs`, `src/model_auth.rs` | `src/model_switchboard_tests.rs` |
| Master key / temporary key persistence | [Local configuration](configuration.md), [Current limitations](current-limitations.md) | `src/master_key*.rs`, `src/config.rs` | `src/master_key_tests.rs`, config master-key test |
| CVA composition / single physical owner | [Architecture](architecture.md), [ADR 0005](decisions/0005-cva-composition-and-packed-vector-objects.md) | `src/cva.rs`, `src/cva_lifecycle.rs`, `src/cva_*.rs`, `src/cva_error.rs` | concrete-store reopen tests |
| CVA divergent reconciliation / safe promotion | [Architecture](architecture.md), [Rust API](api.md), [ADR 0019](decisions/0019-cloud-backed-cva-reconciliation.md) | `src/cva_reconcile*.rs` | `src/cva_reconcile_*tests.rs` |
| REL/PHY header, chunks, file-kind lifecycle | [Storage format](storage-format.md), [Architecture](architecture.md), [ADR 0020](decisions/0020-reliquary-and-phylactery-file-kinds.md) | `src/container.rs`, `src/container_scan.rs`, `src/cva_lifecycle.rs`, `src/phylactery_lifecycle.rs` | `src/container_tests.rs`, `src/reliquary_tests.rs`, `src/phylactery_tests.rs` |
| Global CVA version ordering | [Architecture](architecture.md), [Storage format](storage-format.md) | `src/container_version.rs` | global-version tests |
| Archive-local watermark/record metadata | [Architecture](architecture.md), [ADR 0003](decisions/0003-layered-version-clocks-and-local-ancestry.md) | `src/archive_history.rs`, `src/archive_history_*` | `src/history_tests.rs` |
| Conversation node ancestry / derived leaf inventory / transcript paths / live branch search | [Architecture](architecture.md), [Rust API](api.md) | `src/archive.rs`, `src/archive_store.rs`, `src/archive_conversation.rs`, `src/archive_model.rs`, `src/cva_conversation.rs`, `src/conversation_search*.rs` | `src/conversation_tests.rs`, `src/conversation_search_tests.rs`, Archive/history tests, corpus smoke |
| Branch/session head revisions and old-session revival | [Architecture](architecture.md), [ADR 0003](decisions/0003-layered-version-clocks-and-local-ancestry.md) | `src/archive.rs`, `src/archive_history.rs` | `branch_heads_are_append_only_revisions`, `old_conversation_point_can_start_a_new_local_branch` |
| Content addressing/body reuse | [Architecture](architecture.md), [Storage format](storage-format.md) | `src/archive_store.rs`, `src/archive_codec.rs` | `src/archive_tests.rs` |
| Native source-turn attachments / embedded files / file-Memory links | [Architecture](architecture.md), [Storage format](storage-format.md), [Rust API](api.md) | `src/turn_ingest_*.rs`, `src/source_attachment_index.rs`, `src/file*.rs`, `src/cva_turn_ingest.rs`, `src/cva_file_memory.rs` | `src/turn_ingest_tests.rs`, `src/file_tests.rs`, `src/file_memory_link_tests.rs` |
| Deterministic Episodes / finalization policy | [Architecture](architecture.md), [ADR 0012](decisions/0012-deterministic-episodes-and-insomnia-memory-authority.md) | `src/episode_*.rs` | `src/episode_tests.rs` |
| Memories / revision authority / provenance | [Architecture](architecture.md), [Rust API](api.md), [ADR 0012](decisions/0012-deterministic-episodes-and-insomnia-memory-authority.md), [ADR 0020](decisions/0020-reliquary-and-phylactery-file-kinds.md) | `src/memory_*.rs`, `src/cva_memory_publish.rs`, `src/phylactery.rs` | `src/memory_tests.rs`, `src/phylactery_tests.rs` |
| Graph persistence / relationship topology / traversal | [Architecture](architecture.md), [Storage format](storage-format.md) | `src/graph_*.rs`, `src/cva_graph.rs`, pinned `arcana-graph` crate | `src/graph_tests.rs` + `arcana-graph` tests |
| Derived Graph communities / Leiden scan-and-merge | [Architecture](architecture.md), [Storage format](storage-format.md), [Rust API](api.md), [ADR 0025](decisions/0025-owner-local-derived-communities.md), [Community benchmark](community-scan-merge-benchmark-2026-08-27.md) | `src/community_*.rs`, `src/cva_communities.rs`, `src/phylactery_communities.rs` | `src/community_tests.rs`, `src/community_scan_merge_tests.rs`, ignored `src/community_scan_merge_bench.rs` |
| Owner-local Memory-Web retrieval / Community routing / traversal | [Architecture](architecture.md), [Rust API](api.md), [Current limitations](current-limitations.md), [ADR 0026](decisions/0026-community-routed-memory-retrieval.md), [Community routing validation](community-routing-validation-2026-08-29.md) | `src/memory_retrieval*.rs`, `src/cva_memory_retrieval.rs`, `src/phylactery_memory_retrieval.rs`; benchmark compatibility in `src/community_subcentroid_routing.rs` | `src/memory_retrieval_tests.rs`, ignored `src/community_end_to_end_grid_bench.rs`, ignored `src/community_end_to_end_quality_bench.rs` |
| Dream Memory candidate retrieval | [Architecture](architecture.md), [Rust API](api.md), [Dream implementation plan](dream-implementation-plan.md), [ADR 0024](decisions/0024-owner-local-dream-processing.md) | `src/dream_candidate_*.rs`, `src/dream_candidates.rs`, `src/dream_owner_*.rs`, `src/phylactery_dream_candidates.rs` | `src/dream_candidate_tests.rs`, `src/phylactery_dream_tests.rs` |
| Dream pair classification / direction contract | [Architecture](architecture.md), [Rust API](api.md), [Dream implementation plan](dream-implementation-plan.md) | `src/dream_classifier*.rs`, `src/dream_pair_context.rs`, `src/model_switchboard*.rs`, configured General transports | `src/dream_classifier_tests.rs`, `src/model_switchboard_tests.rs` |
| Dream independent verification / verification policy | [Architecture](architecture.md), [Rust API](api.md), [Dream implementation plan](dream-implementation-plan.md) | `src/dream_verifier*.rs`, `src/dream_pair_context.rs` | `src/dream_verifier_tests.rs` |
| Dream Graph publication / pair reconciliation | [Architecture](architecture.md), [Rust API](api.md), [Storage format](storage-format.md), [Dream implementation plan](dream-implementation-plan.md), [ADR 0024](decisions/0024-owner-local-dream-processing.md) | `src/dream_publisher*.rs`, `src/dream_owner_publisher.rs`, `src/phylactery_dream_publisher.rs`, owner-local Graph APIs | `src/dream_publisher_tests.rs`, `src/phylactery_dream_tests.rs`, Graph batch tests |
| Dream duplicate predecessor chain / source-time index | [Architecture](architecture.md), [Rust API](api.md), [Dream implementation plan](dream-implementation-plan.md), [ADR 0024](decisions/0024-owner-local-dream-processing.md) | `src/dream_duplicate.rs`, `src/dream_owner_duplicate.rs`, `src/dream_duplicate_index.rs`, `src/dream_source_time.rs` | `src/dream_duplicate_tests.rs`, `src/phylactery_dream_tests.rs` |
| Dream lifecycle / bounded end-to-end pass | [Architecture](architecture.md), [Rust API](api.md), [Dream implementation plan](dream-implementation-plan.md), [ADR 0024](decisions/0024-owner-local-dream-processing.md) | `src/dream_lifecycle*.rs`, `src/dream_processor*.rs`, `src/phylactery_dream_*.rs`, `src/runtime_host_dream*.rs` | `src/dream_lifecycle_tests.rs`, `src/dream_processor_tests.rs`, `src/phylactery_dream_tests.rs`, RuntimeHost Dream/vector tests |
| Dream deterministic temporal analysis / temporal candidate lane | [Architecture](architecture.md), [Rust API](api.md), [Dream implementation plan](dream-implementation-plan.md), [Current limitations](current-limitations.md) | `src/dream_temporal*.rs`, `src/dream_candidates.rs`, `src/dream_candidate_ranking.rs`, `src/dream_pair_context.rs` | `src/dream_temporal_tests.rs`, temporal processor/classifier assertions |
| Insomnia extraction / evidence / queue / finite worker | [Architecture](architecture.md), [Current limitations](current-limitations.md), [ADR 0012](decisions/0012-deterministic-episodes-and-insomnia-memory-authority.md) | `src/insomnia.rs`, `src/insomnia/**/*.rs` | `src/insomnia/**/*tests.rs` |
| Memory Vector bindings / missing-only embedding | [Architecture](architecture.md), [Rust API](api.md), [ADR 0013](decisions/0013-immutable-memory-vector-bindings.md) | `src/memory_vector_*.rs`, `src/cva_memory_vectors.rs`, `src/phylactery_memory_vectors.rs` | `src/memory_vector_tests.rs`, `src/phylactery_tests.rs`, `src/insomnia/worker_vector_tests.rs` |
| Live interaction session / stream runtime | [Architecture](architecture.md), [Rust API](api.md), [ADR 0016](decisions/0016-native-product-surface-and-shared-interaction-runtime.md) | `src/interaction_model.rs`, `src/interaction_error.rs`, `src/interaction_runtime.rs`, `src/interaction_session.rs`, `src/interaction_stream.rs` | `src/interaction_runtime_tests.rs`, `src/interaction_session_tests.rs`, `src/conversation_tests.rs` |
| Durable REL/PHY owner identity | [Architecture](architecture.md), [Storage format](storage-format.md), [Rust API](api.md) | `src/container.rs`, `src/container_scan.rs`, `src/cva_lifecycle.rs`, `src/phylactery_lifecycle.rs` | owner-ID tests pending fixture rewrite during format repair |
| Warlock host integration / broader workspace-CVA application surface | [Architecture](architecture.md), [Roadmap](roadmap.md), [ADR 0016](decisions/0016-native-product-surface-and-shared-interaction-runtime.md), [ADR 0017](decisions/0017-cva-workspace-and-warlock-host-application.md) | Reliquary public workspace + interaction APIs consumed by Warlock v2; broader host orchestration remains Warlock-owned | Reliquary workspace/interaction/conversation suites plus Warlock v2 integration tests |
| ACP interoperability adapter | [ADR 0015](decisions/0015-acp-inline-interaction-stream.md), [ADR 0016](decisions/0016-native-product-surface-and-shared-interaction-runtime.md), [Roadmap](roadmap.md) | not implemented | future adapter normalization/capture suites |
| Fragment identity/windows/tails | [Architecture](architecture.md), [Storage format](storage-format.md) | `src/fragment_*` | `src/fragment_tests.rs` |
| Packed-vector representation/storage | [Storage format](storage-format.md), [ADR 0005](decisions/0005-cva-composition-and-packed-vector-objects.md) | `src/packed_vector_*`, Lodestone `crates/packed` | `src/packed_vector_tests.rs`, Lodestone packed tests |
| Archive-Vector row bindings | [Storage format](storage-format.md), [ADR 0006](decisions/0006-archive-vector-row-bindings.md) | `src/archive_vector_*`, `src/cva_archive_vectors.rs` | `src/archive_vector_tests.rs` |
| Embedding endpoint / compatibility contract | [Rust API](api.md), [ADR 0007](decisions/0007-compatibility-profiles-and-vector-generations.md) | `src/embedding_endpoint.rs`, `src/openai_ready_embedding*.rs`, `src/compatibility_profile_*`, `src/cva_compatibility_profiles.rs` | endpoint-response tests + compatibility-profile tests |
| Vector-generation publication/history | [Architecture](architecture.md), [Versioning plan](version-history-plan.md), [ADR 0007](decisions/0007-compatibility-profiles-and-vector-generations.md) | `src/vector_generation_*`, `src/cva_vector_generations.rs` | generation tests + corpus vector smoke |
| Exact semantic retrieval | [Architecture](architecture.md), [Rust API](api.md), [Current limitations](current-limitations.md) | `src/semantic_search*.rs` | `src/semantic_search_tests.rs`, corpus vector/retrieval smoke |
| Default lexical/hybrid retrieval | [Architecture](architecture.md), [Rust API](api.md), [Current limitations](current-limitations.md) | `src/lexical_search.rs`, `src/search*.rs` | `src/search_tests.rs`, corpus vector/retrieval smoke |
| Reopen reconstruction/indexing/checkpoint acceleration | [Architecture](architecture.md), [Roadmap](roadmap.md), [Current limitations](current-limitations.md) | `src/cva_lifecycle.rs`, `src/container.rs`, concrete rebuild states, Archive indexes | reopen tests + heap/startup profiling |
| Whole-CVA restore across databases | [Versioning plan](version-history-plan.md) | not implemented | future multi-store recovery suite |
| Additional semantic databases | [Roadmap](roadmap.md), [ADR 0001](decisions/0001-purpose-built-database-ownership.md) | not implemented | future store-local suites |
| Documentation governance | [Documentation policy](documentation-policy.md) | `docs-standard.json`, `AGENTS.md`, `docs/` | shared documentation checker |

## Boundaries

- `cli/` owns only operator argument/prompt/output composition and depends exclusively on public library APIs; removing it cannot change core library semantics.
- `ReliquaryConfig` owns machine-local current configuration separately from `.rel`; config changes do not enter semantic history.
- `CredentialsConfig` owns decrypted in-memory provider secrets; `credential.<id>` objects are independently authenticated/encrypted and route references use stable IDs.
- `ModelSwitchboardConfig` owns provider/model/endpoint/credential selection; `ModelSwitchboard` validates matching credentials and attaches request auth. These choices cannot establish Compatibility Profile identity or vector compatibility.
- `Cva` owns the single Container handle and explicit concrete-store scan dispatch; it owns no semantic dependency graph.
- Durable owner identity belongs to the typed container header. Scope/type plus UUID derives the canonical owner ID; filenames and optional adapters remain outside core identity.
- Container owns physical storage and global ordering only.
- Archive owns Archive-local ordering and conversation/session semantics.
- Packed-vector storage owns immutable matrix identity/shape only.
- Archive-Vector storage owns row-to-Archive-fragment identity only.
- Compatibility Profiles own immutable endpoint-independent vector-space compatibility contracts and reference probes; provider/model provenance is outside this owner.
- Vector Generations own profile/population activation, Archive coverage, and the vector-local semantic watermark.
- Archive and vector-local version adjacency do not imply ancestry or cross-store dependency identity.
- Node parent links and branch/session revisions own local conversation history.
- Source attachments are intrinsic to source-turn ingestion; later file-to-Memory links remain explicit cross-owner references.
- `InteractionRuntime` owns transport-neutral session coordination, completed-message stream assembly, durable acknowledgement, and explicit live Episode scheduling above Archive ingestion; future adapters and product surfaces must converge through that boundary rather than defining Archive semantics.
- A future CVA management surface composes concrete owner APIs and cannot become a generalized semantic store.
- Future stores retain independent authority even when they share the CVA global clock.

## Related docs

- [Architecture](architecture.md)
- [Documentation coverage](documentation-coverage.md)
- [Behavioral contracts](behavioral-contracts.md)
- [Roadmap](roadmap.md)

## Notes

Update this map when future runtime, management, UI, adapter, or semantic-store work gains concrete implementation roots.