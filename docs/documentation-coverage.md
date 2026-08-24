# Documentation Coverage

Parent index: [Documentation index](INDEX.md)

## Purpose

This document maps current implementation boundaries to canonical documentation owners.

## Overview

Coverage remains intentionally small while the rebuild is early.

## Implementation coverage

| Implementation | Responsibility | Canonical owners |
| --- | --- | --- |
| `cli/src/*.rs`, `cli/Cargo.toml` | Detachable repo-local command package for CVA/config/auth/archive/vector bring-up over the public library API | [Repo-local CLI](cli.md), [Development](development.md), [Architecture](architecture.md), [ADR 0011](decisions/0011-detachable-repo-local-cli.md) |
| `src/config*.rs` | Purpose-built local config framing, typed fragment/retrieval/model/credential objects, validation, encryption integration, atomic replacement, tests | [Local configuration](configuration.md), [Architecture](architecture.md), [Rust API](api.md), [ADR 0008](decisions/0008-purpose-built-local-configuration.md), [ADR 0010](decisions/0010-encrypted-credential-objects.md) |
| `src/credential*.rs` | Credential IDs/types, secret redaction/zeroization, AES-256-GCM object encryption/codecs, tamper/wrong-key tests | [Local configuration](configuration.md), [Architecture](architecture.md), [Rust API](api.md), [ADR 0010](decisions/0010-encrypted-credential-objects.md) |
| `src/model_switchboard*.rs`, `src/model_auth.rs` | General/Embedding capability routing, credential references, provider auth validation, request-header attachment, tests | [Architecture](architecture.md), [Local configuration](configuration.md), [Rust API](api.md), [ADR 0009](decisions/0009-expandable-model-switchboard.md), [ADR 0010](decisions/0010-encrypted-credential-objects.md) |
| `src/openai_ready_embedding*.rs` | Direct OpenAI-compatible embedding HTTP execution, batching/concurrency, response ordering/validation, normalization | [Architecture](architecture.md), [Rust API](api.md), [Current limitations](current-limitations.md) |
| `src/master_key*.rs` | 256-bit master-key generation, temporary JSON persistence, redaction, tests | [Local configuration](configuration.md), [Rust API](api.md), [Current limitations](current-limitations.md), [ADR 0010](decisions/0010-encrypted-credential-objects.md) |
| `src/cva.rs`, `src/cva_lifecycle.rs`, `src/cva_*.rs`, `src/cva_error.rs` | Single-Container composition, concrete-store scan dispatch, public lifecycle/API | [Architecture](architecture.md), [Rust API](api.md), [ADR 0005](decisions/0005-cva-composition-and-packed-vector-objects.md) |
| `src/container.rs` | CVA header/chunks/read/append/sync | [Architecture](architecture.md), [Storage format](storage-format.md), [Rust API](api.md) |
| `src/container_scan.rs` | Single-pass physical payload scan and global-ticket observation | [Architecture](architecture.md), [Storage format](storage-format.md), [Development](development.md) |
| `src/container_version.rs` | CVA-global monotonic version tickets | [Architecture](architecture.md), [Storage format](storage-format.md) |
| `src/container_error.rs` | Physical/version errors | [Storage format](storage-format.md), [Rust API](api.md) |
| `src/container_tests.rs` | Container/version verification | [Behavioral contracts](behavioral-contracts.md) |
| `src/archive.rs` | Archive mutation/read and branch-head revisions | [Architecture](architecture.md), [Rust API](api.md) |
| `src/archive_history.rs` | Archive-local watermark and dual-version metadata | [Architecture](architecture.md), [Versioning plan](version-history-plan.md), [Rust API](api.md) |
| `src/archive_history_codec.rs` | Archive format + record-version metadata codec | [Storage format](storage-format.md) |
| `src/archive_history_model.rs` | `ArchiveRecordVersion` model | [Storage format](storage-format.md), [Rust API](api.md) |
| `src/archive_model.rs` | Archive public semantic models | [Architecture](architecture.md), [Rust API](api.md) |
| `src/archive_codec.rs` | Node/content/branch/fragment codecs | [Storage format](storage-format.md) |
| `src/archive_rebuild.rs` | Single-pass reopen reconstruction and semantic visibility activation | [Architecture](architecture.md), [Storage format](storage-format.md), [Current limitations](current-limitations.md) |
| `src/archive_store.rs` | Content access, branch traversal, and current-state validation | [Architecture](architecture.md), [Current limitations](current-limitations.md) |
| `src/archive_error.rs` | Archive error contract | [Rust API](api.md) |
| `src/archive_tests.rs`, `src/archive_inventory_tests.rs` | Archive dedupe/idempotency plus current branch-inventory read behavior | [Behavioral contracts](behavioral-contracts.md) |
| `src/fragment_*` | Fragment identity/materialization/storage/tests | [Architecture](architecture.md), [Storage format](storage-format.md), [Behavioral contracts](behavioral-contracts.md) |
| `src/history_tests.rs` | Dual clocks, independent conversations, branch revisions/revival | [Behavioral contracts](behavioral-contracts.md), [ADR 0003](decisions/0003-layered-version-clocks-and-local-ancestry.md) |
| `src/packed_vector_*` | Packed-vector format, content identity, derived lookup, reopen, tests | [Architecture](architecture.md), [Storage format](storage-format.md), [Rust API](api.md), [ADR 0005](decisions/0005-cva-composition-and-packed-vector-objects.md) |
| `src/archive_vector_*` | Row-to-FragmentId binding format, identity, reference/coverage derivation, reopen, tests | [Architecture](architecture.md), [Storage format](storage-format.md), [Rust API](api.md), [ADR 0006](decisions/0006-archive-vector-row-bindings.md) |
| `src/embedding_endpoint.rs`, `src/compatibility_profile_*`, `src/cva_compatibility_profiles.rs` | Endpoint abstraction, tolerant compatibility probing, durable compatibility contracts, tests | [Architecture](architecture.md), [Rust API](api.md), [ADR 0007](decisions/0007-compatibility-profiles-and-vector-generations.md) |
| `src/vector_generation_*` | Generation format, local/global ordering, current-per-profile state, validation, reopen, tests | [Architecture](architecture.md), [Storage format](storage-format.md), [Versioning plan](version-history-plan.md), [ADR 0007](decisions/0007-compatibility-profiles-and-vector-generations.md) |
| `src/semantic_search*.rs` | Read-only compatibility-gated current-generation exact cosine retrieval and tests | [Architecture](architecture.md), [Rust API](api.md), [Current limitations](current-limitations.md), [Behavioral contracts](behavioral-contracts.md) |
| `src/lexical_search.rs`, `src/search*.rs` | Original lexical scoring plus default hybrid fusion/diversification/result policy | [Architecture](architecture.md), [Rust API](api.md), [Current limitations](current-limitations.md), [Behavioral contracts](behavioral-contracts.md) |
| `src/lib.rs` | Public crate exports | [Rust API](api.md) |
| `examples/archive_roundtrip.rs` | Prepared-corpus reopen smoke | [Development](development.md), [Behavioral contracts](behavioral-contracts.md) |
| `examples/archive_open_profile.rs` | Standalone open-time/allocator benchmark | [Development](development.md) |
| `examples/vector_generation_smoke.rs` | Two-profile simulated generation build/reopen over prepared corpus | [Development](development.md), [Behavioral contracts](behavioral-contracts.md) |
| `examples/insomnia_two_pass_sol/*`, `examples/run_two_pass_openrouter.rs`, `tools/insomnia_two_pass_prompts.py`, `tools/nous_json_client.py`, `tools/run_insomnia_two_pass.py` | Experimental two-pass Insomnia authority-ledger/synthesis validation plus Ox/Nous/OpenRouter provider compatibility harnesses | [Development](development.md), [Insomnia two-pass validation](insomnia-two-pass-validation-2026-08-24.md), [Roadmap](roadmap.md) |

## Coverage rules

New production modules require canonical owners. New persistence records update storage-format documentation. New stateful/recovery boundaries update architecture and behavioral contracts in the same change.

## Related docs

- [Documentation policy](documentation-policy.md)
- [Maintainer map](maintainer-map.md)
- [Architecture](architecture.md)
- [Behavioral contracts](behavioral-contracts.md)

## Notes

Future components remain planning-only until code exists.