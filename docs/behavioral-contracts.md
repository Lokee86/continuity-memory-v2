# Behavioral Contracts

Parent index: [Documentation index](INDEX.md)

## Purpose

This document maps critical current behavioral invariants to focused tests and smoke verification.

## Overview

The matrix covers implemented behavior. Future cross-database restore and concurrency contracts remain explicit future requirements.

## Contract matrix

| Contract | Protection |
| --- | --- |
| Default local config saves/reopens with Continuity magic and typed defaults | `config_tests::default_config_saves_and_reopens` |
| Config replacement keeps only current objects and does not grow from history | `config_tests::replacing_config_does_not_accumulate_old_objects` |
| Unknown future config objects survive known-object replacement | `config_tests::unknown_objects_survive_known_config_replacement` |
| Invalid config is rejected before replacing the existing file | `config_tests::invalid_values_do_not_replace_existing_config` |
| Switchboard provider auth/capabilities are explicit | `model_switchboard_tests::provider_auth_and_capabilities_are_explicit` |
| General/embedding routes and encrypted credentials round-trip together; route removal remains current-state replacement | `model_switchboard_tests::routes_and_credentials_round_trip_and_attach_auth_headers`, `model_switchboard_tests::clearing_model_routes_removes_them_from_current_config` |
| Unsupported provider routes plus missing/wrong-kind credentials are rejected | `model_switchboard_tests::invalid_provider_routes_are_rejected`, `model_switchboard_tests::missing_or_wrong_credential_kind_is_rejected` |
| OpenAI-ready and Codex credentials resolve to bearer auth; Codex also attaches ChatGPT account ID when present | `model_switchboard_tests::routes_and_credentials_round_trip_and_attach_auth_headers` |
| Credential plaintext never appears in `continuity.cfg` and encrypted credentials round-trip | `credential_tests::encrypted_credentials_round_trip_without_plaintext_in_config` |
| Wrong master keys and ciphertext tampering fail authenticated decryption | `credential_tests::wrong_master_key_cannot_decrypt_credentials`, `credential_tests::authenticated_encryption_rejects_tampering` |
| Clearing credentials removes encrypted credential objects rather than accumulating history | `credential_tests::clearing_credentials_removes_encrypted_objects` |
| Master key is generated once from OS entropy and remains stable across reloads | `master_key_tests::json_store_generates_and_reloads_one_stable_key` |
| Independent key stores produce different keys and debug output redacts key material | `master_key_tests::separately_created_stores_get_different_keys`, `master_key_tests::debug_output_never_contains_key_material` |
| `ContinuityConfig` places the temporary JSON master key beside the config | `config_tests::config_creates_temporary_master_key_beside_itself` |
| New CVA header reopens | `container_tests::create_then_reopen_cva` |
| Opaque chunks retain stable references | `container_tests::append_then_read_chunks` |
| Global versions survive reopen | `container_tests::global_versions_survive_reopen` |
| Truncated/non-CVA files are rejected | `container_tests::*truncated*`, `reject_non_cva_file` |
| Archive requires current format marker | `history_tests::archive_rejects_container_without_archive_format_marker` |
| Unrelated conversations share ordering clocks but not ancestry | `history_tests::unrelated_conversations_share_clocks_not_ancestry` |
| Global and Archive clocks can diverge cleanly | `history_tests::global_and_archive_clocks_are_independent` |
| Archive-local watermark survives reopen | `history_tests::archive_versions_survive_reopen` |
| Branch/session heads are append-only revisions with historical lookup | `history_tests::branch_heads_are_append_only_revisions` |
| Current branch inventory exposes only the latest branch revision after reopen | `archive_inventory_tests::current_branch_inventory_exposes_latest_revision_after_reopen` |
| Existing branch heads cannot jump backward to an ancestor | `history_tests::existing_branch_head_cannot_jump_backwards` |
| An old conversation point can seed a new local branch without Archive rollback | `history_tests::old_conversation_point_can_start_a_new_local_branch` |
| Shared prefixes/content bodies deduplicate | `archive_tests::shared_branch_prefix_and_content_are_stored_once` |
| Identical node append is idempotent | `archive_tests::identical_node_append_is_idempotent` |
| Unversioned semantic payloads remain inert on reopen | `archive_tests::unversioned_semantic_record_is_inert_on_reopen` |
| Fragment windows/tails remain append-only and branch-neutral | `fragment_tests::*` |
| Packed-vector matrices round-trip beside Archive data in one CVA | `packed_vector_tests::packed_vectors_round_trip_inside_same_cva_as_archive` |
| Equal packed matrices deduplicate by schema+bytes | `packed_vector_tests::identical_packed_matrix_is_content_addressed_once` |
| Raw packed-vector backing objects consume no semantic clock | `packed_vector_tests::raw_packed_vectors_do_not_advance_semantic_clocks` |
| Packed representation handles 512×int8 through 4096×float64 without special-case layouts | `packed_vector_tests::supports_large_float64_rows_without_special_cases`, Lodestone packed tests |
| Archive Vectors preserve exact row-to-FragmentId order across reopen | `archive_vector_tests::archive_vectors_bind_rows_to_fragments_and_round_trip` |
| Equal Archive-Vector bindings deduplicate by matrix ID + ordered fragments | `archive_vector_tests::identical_archive_vector_binding_is_content_addressed_once` |
| Archive Vectors require an existing matrix and exact row count | `archive_vector_tests::binding_requires_existing_matrix_and_exact_row_count` |
| Archive Vector mappings require real unique fragments | `archive_vector_tests::binding_requires_real_unique_fragments` |
| Archive Vector backing objects consume no semantic clock | `archive_vector_tests::archive_vector_objects_do_not_advance_semantic_clocks` |
| Corrupt Archive Vector mappings are rejected on reopen | `archive_vector_tests::corrupt_archive_vector_mapping_is_rejected_on_reopen` |
| Compatibility Profiles round-trip and consume no semantic clock | `compatibility_profile_tests::profile_round_trips_and_is_clock_neutral` |
| Small numerical drift remains compatible and reuses the existing profile | `compatibility_profile_tests::tiny_endpoint_drift_is_compatible_and_reuses_profile` |
| Materially different endpoint behavior creates a distinct compatibility profile | `compatibility_profile_tests::materially_different_endpoint_gets_a_distinct_profile` |
| Dimension mismatch is incompatible before vector comparison | `compatibility_profile_tests::dimensions_are_part_of_compatibility` |
| Simulated endpoint builds/reopens an active vector generation | `vector_generation_tests::simulated_endpoint_builds_and_reopens_active_generation` |
| Rebuilding unchanged profile/Archive population is idempotent | `vector_generation_tests::repeated_build_is_idempotent` |
| Newer Archive cuts supersede only that profile's current generation | `vector_generation_tests::newer_archive_cut_supersedes_compatibility_profile_generation` |
| Different compatibility profiles retain independent current generations | `vector_generation_tests::compatibility_profiles_keep_independent_active_generations` |
| Small compatible endpoint drift can build under an existing profile | `vector_generation_tests::compatible_endpoint_drift_can_build_under_existing_profile` |
| Incompatible endpoint behavior cannot build under a profile | `vector_generation_tests::incompatible_endpoint_cannot_build_under_profile` |
| Profile/packed dimensions must match | `vector_generation_validation_tests::publication_rejects_profile_matrix_dimension_mismatch` |
| Published generations reject non-`f32` matrices until alternate scalar semantics exist | `vector_generation_validation_tests::publication_rejects_non_f32_generation_matrix` |
| Generation source cut must cover every mapped fragment | `vector_generation_validation_tests::publication_rejects_source_before_mapped_fragments` |
| One global version cannot be claimed by both Archive and Vector Generations | `vector_generation_validation_tests::reopen_rejects_global_version_claimed_by_archive_and_vectors` |
| Unversioned generation payloads are inert | `vector_generation_validation_tests::unversioned_generation_payload_is_inert_on_reopen` |
| Exact semantic search verifies compatibility, uses Query mode, ranks by cosine, drops non-positive scores, advances no semantic clock, and survives reopen | `semantic_search_tests::semantic_search_exactly_ranks_current_generation_and_reopens` |
| Semantic search resolves the newest generation for the selected profile | `semantic_search_tests::semantic_search_uses_newest_generation_for_profile` |
| Semantic search rejects missing generations, empty/invalid limits, and incompatible endpoints | `semantic_search_tests::semantic_search_rejects_invalid_requests_and_incompatible_endpoint` |
| Default search restores `0.45/0.55` fusion, 30 candidates, 10 results, undiluted single-channel scores, and accepts explicit validated retrieval policy | `search_policy_tests::old_default_weights_and_limits_are_restored`, `search_tests::hybrid_search_preserves_single_channel_scores_and_blends_shared_hits`, `search_tests::duplicate_ranges_are_removed_and_default_result_limit_is_ten` |
| Lexical scoring preserves the original coverage+density formula | `search_policy_tests::lexical_scoring_matches_original_coverage_density_formula` |
| Same-conversation overlapping ranges are diversified | `search_tests::diversification_penalizes_overlapping_ranges_from_same_conversation` |
| Duplicate ranges are removed and default output is capped at 10 | `search_tests::duplicate_ranges_are_removed_and_default_result_limit_is_ten` |
| Prepared corpus round-trips branches/content/fragments | `examples/archive_roundtrip.rs` |
| Insomnia structured extraction requires explicit candidate authority kind (`direct`/`correction`/`adoption`/`retention`) and preserves it through candidate parsing | `insomnia::extraction_tests::authority_kind_is_required_and_preserved_by_structured_extraction` |
| Insomnia deterministic authority policy distinguishes assistant authority provenance from referent grounding, rejects invalid authority/provenance combinations and unsupported vague/question authority, and preserves user-owned corrections such as ShipStats when grounding resolves only the referent | `insomnia::candidate_policy_tests::*`, `insomnia::grounding_tests::correction_can_use_grounding_without_adopting_assistant_authority` |
| Insomnia rejects pure execution receipts and residual numbered prompt/phase/step progress; useful resulting state is accepted only after progress framing is removed | `insomnia::candidate_receipt_policy_tests::*` |
| Current Insomnia gold v2 contains exactly 40 unique source-anchored judgments (32 retain / 8 omit) with separate authority-source and grounding-source policy; four cases explicitly require grounding without assistant authority | `insomnia::gold_tests::insomnia_gold_set_is_well_formed_and_balanced` |
| Prepared corpus supports two independent simulated profiles/generations plus default hybrid retrieval | `examples/vector_generation_smoke.rs` |

## Future contracts

Before concurrent writers are supported, tests must prove that unrelated sessions do not serialize on a semantic Archive head and that global/file-position reservation is the only required shared ordering boundary.

Before whole-CVA rollback ships, tests must define timeline selection across Archive, Memories, Graph, and vector domains without turning every write into a global state publication.

Before persistent checkpoints are added, larger-scale benchmarks must show that the remaining single-pass scan cost justifies the additional derived-state machinery.

## Related docs

- [Architecture](architecture.md)
- [Architectural invariants](invariants.md)
- [Versioning and rollback plan](version-history-plan.md)
- [Development](development.md)

## Notes

A tracked integer watermark is not by itself a historical materialization API; tests distinguish stored ordering metadata from features not yet exposed.