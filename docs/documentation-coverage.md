# Documentation Coverage

Parent index: [Documentation index](INDEX.md)

## Purpose

This document maps implemented code boundaries to their canonical documentation owners.

## Overview

Only implemented code belongs in this matrix. Future-only product/runtime work is owned by [Roadmap](roadmap.md) and is intentionally excluded until concrete implementation roots exist.

## Implementation coverage

| Implementation | Responsibility | Canonical owners |
| --- | --- | --- |
| `cli/src/*.rs`, `cli/Cargo.toml` | Detachable repo-local command package for CVA/config/auth/archive/vector/Insomnia bring-up over the public library API | [Repo-local CLI](cli.md), [Development](development.md), [Architecture](architecture.md), [ADR 0011](decisions/0011-detachable-repo-local-cli.md) |
| `src/config*.rs` | Purpose-built local config framing, typed fragment/retrieval/model/credential objects, validation, encryption integration, atomic replacement | [Local configuration](configuration.md), [Architecture](architecture.md), [Rust API](api.md), [ADR 0008](decisions/0008-purpose-built-local-configuration.md), [ADR 0010](decisions/0010-encrypted-credential-objects.md) |
| `src/credential*.rs`, `src/master_key*.rs` | Credential IDs/types, secret redaction/zeroization, AES-256-GCM object encryption, temporary master-key persistence | [Local configuration](configuration.md), [Rust API](api.md), [Current limitations](current-limitations.md), [ADR 0010](decisions/0010-encrypted-credential-objects.md) |
| `src/model_switchboard*.rs`, `src/model_auth.rs`, `src/configured_general.rs` | General/Insomnia/Embedding capability routing, provider auth validation, configured General dispatch | [Architecture](architecture.md), [Local configuration](configuration.md), [Rust API](api.md), [ADR 0009](decisions/0009-expandable-model-switchboard.md) |
| `src/openai_ready_embedding*.rs`, `src/openai_ready_general.rs` | OpenAI-compatible embedding/General HTTP execution and validation | [Architecture](architecture.md), [Rust API](api.md), [Current limitations](current-limitations.md) |
| `src/openai_codex_device_auth*.rs`, `src/openai_codex_general*.rs` | ChatGPT/Codex device authorization plus provider-native General/Insomnia Responses transport | [Architecture](architecture.md), [Rust API](api.md), [Current limitations](current-limitations.md) |
| `src/cva.rs`, `src/cva_lifecycle.rs`, `src/cva_*.rs`, `src/cva_error.rs` | Single-Container composition, concrete-store scan dispatch, composition validation, public lifecycle/API | [Architecture](architecture.md), [Rust API](api.md), [ADR 0005](decisions/0005-cva-composition-and-packed-vector-objects.md) |
| `src/container*.rs` | CVA header, chunk framing, append/read/sync, global version tickets, scan/recovery | [Architecture](architecture.md), [Storage format](storage-format.md), [Rust API](api.md) |
| `src/workspace_metadata*.rs`, `src/cva_workspace.rs`, `src/workspace_metadata_tests.rs` | Optional singleton CVA workspace ID/name/type, one-time initialization, reopen validation, and public workspace lifecycle helpers | [Architecture](architecture.md), [Storage format](storage-format.md), [Rust API](api.md), [ADR 0017](decisions/0017-cva-workspace-and-warlock-host-application.md), [Behavioral contracts](behavioral-contracts.md) |
| `src/archive*.rs`, `src/cva_conversation.rs`, `src/conversation_tests.rs` | Archive source-history ownership, nodes, branches, derived conversation/leaf inventory, exact-leaf transcript resolution, dual Archive/global publication metadata, reopen/indexing | [Architecture](architecture.md), [Storage format](storage-format.md), [Rust API](api.md), [ADR 0003](decisions/0003-layered-version-clocks-and-local-ancestry.md) |
| `src/turn_ingest_*.rs`, `src/source_attachment_index.rs`, `src/cva_turn_ingest.rs` | Atomic source-turn ingestion with intrinsic attached-file provenance | [Architecture](architecture.md), [Storage format](storage-format.md), [Rust API](api.md), [Architectural invariants](invariants.md) |
| `src/interaction_model.rs`, `src/interaction_error.rs`, `src/interaction_runtime.rs`, `src/interaction_session.rs`, `src/interaction_stream.rs`, `src/interaction_runtime_tests.rs`, `src/interaction_session_tests.rs` | Transport-neutral user/agent turn contract, explicit session resume, stream assembly, derived conversation reads, Archive normalization, durable acknowledgement, and separate live Episode scheduling | [Architecture](architecture.md), [Rust API](api.md), [Current limitations](current-limitations.md), [ADR 0016](decisions/0016-native-product-surface-and-shared-interaction-runtime.md), [Behavioral contracts](behavioral-contracts.md) |
| `src/file_*.rs`, `src/cva_file_memory.rs` | Standalone embedded files, filename search inventory, file bytes, file-to-Memory links | [Architecture](architecture.md), [Storage format](storage-format.md), [Rust API](api.md), [Current limitations](current-limitations.md) |
| `src/fragment*.rs` | Fragment identity/materialization/storage and branch-neutral ranges | [Architecture](architecture.md), [Storage format](storage-format.md), [Behavioral contracts](behavioral-contracts.md) |
| `src/episode_*.rs` | Deterministic Episode identity, packing, finalization policy, persistence | [Architecture](architecture.md), [ADR 0012](decisions/0012-deterministic-episodes-and-insomnia-memory-authority.md), [Behavioral contracts](behavioral-contracts.md) |
| `src/memory_*.rs`, `src/cva_memory_publish.rs` | Memory bodies, immutable revisions, publication/versioning, provenance validation | [Architecture](architecture.md), [Rust API](api.md), [ADR 0012](decisions/0012-deterministic-episodes-and-insomnia-memory-authority.md) |
| `src/insomnia.rs`, `src/insomnia/**/*.rs` | Extraction, authority policy, bounded evidence, operational scheduling, atomic completion, finite concurrent worker | [Architecture](architecture.md), [Current limitations](current-limitations.md), [ADR 0012](decisions/0012-deterministic-episodes-and-insomnia-memory-authority.md), [Behavioral contracts](behavioral-contracts.md) |
| `src/packed_vector_*.rs` | Packed-vector format, immutable content identity, reopen/lookup | [Architecture](architecture.md), [Storage format](storage-format.md), [Rust API](api.md), [ADR 0005](decisions/0005-cva-composition-and-packed-vector-objects.md) |
| `src/memory_vector_*.rs`, `src/cva_memory_vectors.rs` | Immutable `(CompatibilityProfileId, MemoryBodyId)` vector bindings and missing-only fill | [Architecture](architecture.md), [Storage format](storage-format.md), [Rust API](api.md), [ADR 0013](decisions/0013-immutable-memory-vector-bindings.md) |
| `src/archive_vector_*.rs`, `src/cva_archive_vectors.rs` | Immutable packed-row to `FragmentId` bindings and coverage derivation | [Architecture](architecture.md), [Storage format](storage-format.md), [Rust API](api.md), [ADR 0006](decisions/0006-archive-vector-row-bindings.md) |
| `src/embedding_endpoint.rs`, `src/compatibility_profile_*.rs`, `src/cva_compatibility_profiles.rs` | Endpoint abstraction, tolerant compatibility probing, durable compatibility contracts | [Architecture](architecture.md), [Rust API](api.md), [ADR 0007](decisions/0007-compatibility-profiles-and-vector-generations.md) |
| `src/vector_generation_*.rs`, `src/cva_vector_generations.rs` | Generation publication, local/global ordering, current-per-profile state, historical lookup/validation | [Architecture](architecture.md), [Storage format](storage-format.md), [Rust API](api.md), [ADR 0007](decisions/0007-compatibility-profiles-and-vector-generations.md) |
| `src/lexical_index.rs`, `src/lexical_search.rs`, `src/search*.rs` | Disposable Archive/file lexical indexes plus default hybrid retrieval policy | [Architecture](architecture.md), [Rust API](api.md), [Current limitations](current-limitations.md), [Behavioral contracts](behavioral-contracts.md) |
| `src/semantic_search*.rs` | Read-only compatibility-gated current-generation exact cosine retrieval | [Architecture](architecture.md), [Rust API](api.md), [Current limitations](current-limitations.md), [Behavioral contracts](behavioral-contracts.md) |
| `src/lib.rs` | Public crate exports | [Rust API](api.md) |
| `examples/archive_roundtrip.rs` | Prepared graph-corpus Archive round-trip smoke | [Development](development.md), [Behavioral contracts](behavioral-contracts.md) |
| `examples/archive_open_profile.rs` | Standalone open-time/allocator benchmark | [Development](development.md) |
| `examples/vector_generation_smoke.rs` | Simulated compatibility/generation/retrieval smoke | [Development](development.md), [Behavioral contracts](behavioral-contracts.md) |
| `examples/insomnia_three_pass_sol_luna/*` | Frozen three-model-pass Insomnia tuning harness: Sol-low semantic ledger, deterministic fixed groups, Luna-low metadata, Sol-low wording | [Development](development.md), [Insomnia semantic validation](insomnia-semantic-validation-2026-08-24.md), [Roadmap](roadmap.md) |
| `examples/run_two_pass_openrouter.rs`, `tools/insomnia_two_pass_prompts.py`, `tools/nous_json_client.py`, `tools/run_insomnia_two_pass.py` | Historical two-pass Ox/Nous/OpenRouter provider-compatibility harnesses sharing the current ledger/synthesis semantic rules | [Development](development.md), [Insomnia semantic validation](insomnia-semantic-validation-2026-08-24.md) |

## Coverage rules

New production modules require canonical owners. New persistence records update storage-format documentation. New stateful/recovery boundaries update architecture and behavioral contracts in the same change. New product/runtime code must move from roadmap-only status into this matrix when it becomes concrete.

## Related docs

- [Documentation policy](documentation-policy.md)
- [Maintainer map](maintainer-map.md)
- [Architecture](architecture.md)
- [Behavioral contracts](behavioral-contracts.md)
- [Roadmap](roadmap.md)

## Notes

Planning-only components are intentionally absent from implementation coverage.
