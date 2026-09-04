pub mod archive;
mod archive_codec;
mod archive_conversation;
mod archive_conversation_metadata;
mod archive_error;
mod archive_history;
mod archive_history_codec;
mod archive_history_model;
mod archive_lookup;
mod archive_model;
mod archive_object_index;
mod archive_rebuild;
mod archive_record_index;
mod archive_search;
mod archive_store;
mod archive_vector_codec;
mod archive_vector_error;
mod archive_vector_model;
mod archive_vector_rebuild;
mod archive_vector_store;
mod community_codec;
mod community_error;
mod community_leiden;
mod community_model;
#[cfg(test)]
mod community_routing;
mod community_scan_merge;
mod community_scan_merge_plan;
mod community_scan_merge_reduce;
mod community_scan_merge_workers;
mod community_store;
mod compatibility_profile_codec;
mod compatibility_profile_error;
mod compatibility_profile_model;
mod compatibility_profile_probe;
mod compatibility_profile_rebuild;
mod compatibility_profile_store;
mod compatibility_vector;
pub mod config;
mod config_codec;
mod config_credentials;
mod config_error;
mod config_io;
mod config_object;
mod configured_general;
mod configured_runtime;
pub mod container;
mod container_error;
mod container_version;
mod conversation_compaction_allocator;
mod conversation_compaction_codec;
mod conversation_compaction_model;
mod conversation_compaction_store;
mod conversation_metadata_codec;
mod conversation_metadata_index;
mod conversation_metadata_model;
mod conversation_search;
mod conversation_search_model;
mod credential;
mod credential_codec;
mod credential_crypto;
pub mod cva;
mod cva_archive_vectors;
mod cva_communities;
mod cva_compatibility_profiles;
mod cva_conversation;
mod cva_error;
mod cva_file_memory;
mod cva_global_validation;
mod cva_graph;
mod cva_lifecycle;
mod cva_memory_publish;
mod cva_memory_retrieval;
mod cva_memory_vectors;
mod cva_packed_vectors;
mod cva_reconcile;
mod cva_reconcile_archive;
mod cva_reconcile_conflict;
mod cva_reconcile_conflict_map;
mod cva_reconcile_error;
mod cva_reconcile_graph;
mod cva_reconcile_interaction;
mod cva_reconcile_memory;
mod cva_reconcile_promote;
mod cva_reconcile_repack;
mod cva_turn_ingest;
mod cva_vector_generations;
mod cva_vector_recovery;
mod dream_candidate_error;
mod dream_candidate_model;
mod dream_candidate_ranking;
mod dream_candidates;
mod dream_canonical;
mod dream_classifier;
mod dream_classifier_error;
mod dream_classifier_model;
mod dream_classifier_schema;
mod dream_duplicate_index;
mod dream_lifecycle;
mod dream_lifecycle_error;
mod dream_lifecycle_model;
mod dream_owner_candidates;
mod dream_owner_duplicate;
mod dream_owner_publisher;
mod dream_owner_vectors;
mod dream_pair_context;
mod dream_processor;
mod dream_processor_error;
mod dream_processor_frontier;
mod dream_processor_model;
mod dream_publisher;
mod dream_publisher_error;
mod dream_publisher_model;
mod dream_source_time;
mod dream_temporal;
mod dream_temporal_absolute;
mod dream_temporal_absolute_calendar;
mod dream_temporal_calendar;
mod dream_temporal_match;
mod dream_temporal_model;
mod dream_temporal_parser;
mod dream_temporal_recurrence;
mod dream_temporal_relative;
mod dream_verifier;
mod dream_verifier_error;
mod dream_verifier_model;
mod dream_verifier_schema;
mod echo_codec;
mod echo_model;
mod echo_store;
mod embedding_endpoint;
mod episode_builder;
mod episode_codec;
mod episode_index;
mod episode_model;
mod episode_policy;
mod episode_store;
mod file_index;
mod file_memory_link_codec;
mod file_memory_link_index;
mod file_memory_link_model;
mod file_memory_link_store;
mod file_model;
mod file_store;
mod fragment_model;
mod fragment_store;
mod fragmenter;
mod general_endpoint;
mod graph_codec;
mod graph_error;
mod graph_model;
mod graph_query;
mod graph_rebuild;
mod graph_store;
mod insomnia;
mod interaction_background;
mod interaction_error;
mod interaction_model;
mod interaction_runtime;
mod interaction_session;
mod interaction_stream;
mod interaction_stream_codec;
mod interaction_stream_persistence;
mod interaction_stream_record;
mod interaction_stream_store;
mod lexical_index;
mod lexical_search;
mod master_key;
mod master_key_entropy;
mod memory_codec;
mod memory_codec_scalar;
mod memory_error;
mod memory_model;
mod memory_provenance_model;
mod memory_rebuild;
mod memory_retrieval;
mod memory_retrieval_build;
mod memory_retrieval_error;
mod memory_retrieval_index;
mod memory_retrieval_model;
mod memory_retrieval_traversal;
mod memory_retrieval_vectors;
mod memory_store;
mod memory_vector_codec;
mod memory_vector_error;
mod memory_vector_model;
mod memory_vector_rebuild;
mod memory_vector_store;
pub mod migration;
mod model_auth;
mod model_switchboard;
mod model_switchboard_codec;
mod openai_codex_device_auth;
mod openai_codex_general;
mod openai_ready_embedding;
mod openai_ready_embedding_response;
mod openai_ready_general;
mod packed_vector_codec;
mod packed_vector_error;
mod packed_vector_model;
mod packed_vector_rebuild;
mod packed_vector_store;
pub mod phylactery;
mod phylactery_communities;
mod phylactery_compatibility_profiles;
mod phylactery_dream_candidates;
mod phylactery_dream_canonical;
mod phylactery_dream_lifecycle;
mod phylactery_dream_publisher;
mod phylactery_error;
mod phylactery_graph;
mod phylactery_lifecycle;
mod phylactery_memory_retrieval;
mod phylactery_memory_vectors;
mod phylactery_packed_vectors;
mod project_file_binding_codec;
mod project_file_binding_store;
mod project_history_codec;
mod project_history_model;
mod project_history_store;
mod runtime_host;
mod runtime_vector_step;
mod search;
mod search_error;
mod search_model;
mod semantic_search;
mod semantic_search_error;
mod semantic_search_model;
mod source_attachment_index;
mod turn_ingest_codec;
mod turn_ingest_model;
mod turn_ingest_store;
mod vector_generation_codec;
mod vector_generation_error;
mod vector_generation_model;
mod vector_generation_rebuild;
mod vector_generation_store;
mod vector_generation_validation;

pub use archive::Archive;
pub use archive_error::ArchiveError;
pub use archive_history_model::ArchiveRecordVersion;
pub use archive_model::{ArchiveStats, Branch, ContentId, ConversationSummary, Node, ResolvedTurn};
pub use archive_search::{
    ArchiveSearchHit, MAX_ARCHIVE_SEARCH_QUERY_BYTES, MAX_ARCHIVE_SEARCH_RESULTS,
};
pub use archive_vector_error::ArchiveVectorError;
pub use archive_vector_model::{
    ArchiveVectorId, ArchiveVectorInfo, ArchiveVectorSet, ArchiveVectorStats,
};
pub use community_error::CommunityError;
pub use community_model::{
    COMMUNITY_ALGORITHM_VERSION, COMMUNITY_LEIDEN_RESOLUTION, COMMUNITY_LEIDEN_SEED, Community,
    CommunityId, CommunitySnapshot, CommunityStats,
};
pub use compatibility_profile_error::CompatibilityProfileError;
pub use compatibility_profile_model::{
    COMPATIBILITY_MIN_COSINE, COMPATIBILITY_POLICY_VERSION, COMPATIBILITY_PROBE_SUITE_VERSION,
    CompatibilityProbeReference, CompatibilityProfile, CompatibilityProfileId,
    CompatibilityProfileStats, CompatibilityReport,
};
pub use config::ReliquaryConfig;
pub use config_error::ConfigError;
pub use configured_general::ConfiguredGeneralEndpoint;
pub use configured_runtime::{
    ArchiveVectorBuildReport, ConfiguredInsomniaOptions, ConfiguredInsomniaReport,
    ConfiguredRuntime, ConfiguredRuntimeError, EmbeddingProbeReport, InsomniaTerminalFailure,
};
pub use container::{
    Container, ContainerError, ContainerIdentity, FileKind, FormatVersion, ObjectRef,
    ReliquaryScopeKind,
};
pub use conversation_compaction_model::{
    ConversationCompaction, ConversationCompactionError, MAX_CONVERSATION_COMPACTION_SUMMARY_BYTES,
};
pub use conversation_metadata_model::ConversationMetadata;
pub use conversation_search_model::ConversationSearchHit;
pub use credential::{Credential, CredentialError, CredentialId, CredentialsConfig, SecretString};
pub use cva::Cva;
pub type Reliquary = Cva;
pub use cva_error::CvaError;
pub use cva_reconcile::{CvaComparison, CvaReconcileResult, CvaRelation};
pub use cva_reconcile_conflict::CvaReconcileConflict;
pub use cva_reconcile_error::CvaReconcileError;
pub use cva_vector_recovery::DerivedVectorRecovery;
pub use dream_candidate_error::DreamCandidateError;
pub use dream_candidate_model::{
    DEFAULT_DREAM_CANDIDATE_LIMIT, DEFAULT_DREAM_LEXICAL_LIMIT, DEFAULT_DREAM_PRIOR_SEMANTIC_QUOTA,
    DEFAULT_DREAM_SEMANTIC_LIMIT, DEFAULT_DREAM_TEMPORAL_LIMIT, DreamCandidate,
    DreamCandidateConfig, DreamCandidateSet, DreamMemoryContext, MAX_DREAM_CANDIDATE_LIMIT,
};
pub use dream_classifier::DreamClassifier;
pub use dream_classifier_error::DreamClassificationError;
pub use dream_classifier_model::{
    DREAM_CLASSIFIER_CONTRACT_VERSION, DreamEvidenceSide, DreamPairClassification,
    DreamPairEvidence, DreamRelationDirection, DreamRelationKind,
};
pub use dream_classifier_schema::{DREAM_CLASSIFIER_SYSTEM_PROMPT, dream_classifier_schema};
pub use dream_lifecycle_error::DreamLifecycleError;
pub use dream_lifecycle_model::DreamLifecycleResult;
pub use dream_processor::DreamProcessor;
pub use dream_processor_error::DreamProcessError;
pub use dream_processor_frontier::{
    DEFAULT_DREAM_FRONTIER_SIZE, DEFAULT_DREAM_INFERENCE_CONCURRENCY, DreamMemoryProcessOutcome,
};
pub use dream_processor_model::{DreamProcessResult, DreamProcessedPair};
pub use dream_publisher_error::DreamPublicationError;
pub use dream_publisher_model::DreamPublicationOutcome;
pub use dream_temporal_model::{
    DreamTemporalAnalysis, DreamTemporalAnchor, DreamTemporalFrequency, DreamTemporalGranularity,
    DreamTemporalMatch, DreamTemporalMatchKind, DreamTemporalOrigin, DreamTemporalPattern,
    DreamTemporalWeekday,
};
pub use dream_verifier::DreamVerifier;
pub use dream_verifier_error::DreamVerificationError;
pub use dream_verifier_model::{
    DREAM_VERIFIER_CONTRACT_VERSION, DreamPairVerification, DreamVerificationPolicy,
    DreamVerificationSignal, DreamVerificationVerdict,
};
pub use dream_verifier_schema::{DREAM_VERIFIER_SYSTEM_PROMPT, dream_verifier_schema};
pub use echo_model::{EchoError, EchoEvent, EchoEventKind};
pub use embedding_endpoint::{
    EmbeddingEndpoint, EmbeddingEndpointError, EmbeddingMode, SimulatedEmbeddingEndpoint,
    VectorNormalization,
};
pub use episode_model::{
    DEFAULT_EPISODE_MAX_INPUT_BYTES, Episode, EpisodeBoundary, EpisodeBuildResult, EpisodeConfig,
    EpisodeId, EpisodeOrigin,
};
pub use episode_policy::{DEFAULT_EPISODE_INACTIVITY_NS, EpisodePolicy};
pub use file_memory_link_model::FileMemoryLink;
pub use file_model::{FileId, FileSearchHit, StoredFile};
pub use fragment_model::{
    DEFAULT_FRAGMENT_OVERLAP, DEFAULT_FRAGMENT_TURNS, Fragment, FragmentConfig, FragmentId,
};
pub use general_endpoint::{GeneralEndpoint, GeneralEndpointError, SimulatedGeneralEndpoint};
pub use graph_error::GraphError;
pub(crate) use graph_model::GraphNodeRecord;
pub use graph_model::{
    GraphDirection, GraphNeighbor, GraphRelation, GraphRelationChange, GraphRelationKind,
    GraphStats, MemoryGraphPath,
};
pub use insomnia::{
    DEFAULT_INSOMNIA_BACKPRESSURE_DELAY_NS, DEFAULT_INSOMNIA_LEASE_NS,
    DEFAULT_INSOMNIA_MAX_ATTEMPTS, DEFAULT_INSOMNIA_POLL_NS,
    DEFAULT_INSOMNIA_PROGRESS_INTERVAL_SECS, DEFAULT_INSOMNIA_RETRY_DELAY_NS,
    DEFAULT_INSOMNIA_SCOPE, DEFAULT_INSOMNIA_WORKERS, EpisodeSchedulingResult,
    INSOMNIA_EXTRACTOR_CONTRACT_VERSION, INSOMNIA_OWNERSHIP_SYSTEM_PROMPT, INSOMNIA_SYSTEM_PROMPT,
    InsomniaAttempt, InsomniaCandidate, InsomniaDrainResult, InsomniaError, InsomniaEvidenceResult,
    InsomniaEvidenceTurn, InsomniaExtraction, InsomniaExtractionError, InsomniaExtractor,
    InsomniaLeaseToken, InsomniaOwnership, InsomniaPriority, InsomniaProcessError,
    InsomniaProcessResult, InsomniaProgressEvent, InsomniaProgressReporter, InsomniaRejection,
    InsomniaSemanticStage, InsomniaStats, InsomniaWork, InsomniaWorkState, InsomniaWorkerConfig,
    InsomniaWorkerError, MAX_INSOMNIA_CANDIDATES, MAX_INSOMNIA_EVIDENCE_BYTES,
    MAX_INSOMNIA_EVIDENCE_REQUESTS, MAX_INSOMNIA_EVIDENCE_TURNS, MAX_INSOMNIA_WORKERS,
    insomnia_schema,
};
pub use interaction_background::{
    DEFAULT_RUNTIME_DREAM_BATCH, RuntimeBackgroundConfig, RuntimeBackgroundError,
    RuntimeBackgroundResult, RuntimeDreamCompletion, RuntimeDreamFailure,
};
pub use interaction_error::InteractionError;
pub use interaction_model::{
    InteractionAttachment, InteractionRole, InteractionSession, InteractionTurn,
};
pub use interaction_runtime::{InteractionCompletion, InteractionReceipt, InteractionRuntime};
pub use interaction_stream_record::{
    InteractionStreamRecord, InteractionStreamStatus, InteractionTurnStatus,
    ResolvedInteractionTurn,
};
pub use lodestone_packed::{PackedVectors, ScalarType, VectorSchema};
pub use master_key::{
    JsonMasterKeyStore, MASTER_KEY_BYTES, MasterKey, MasterKeyError, MasterKeyStore,
};
pub use memory_error::MemoryError;
pub use memory_model::{
    Memory, MemoryBodyId, MemoryDraft, MemoryId, MemoryRef, MemoryRevisionId, MemoryStats,
};
pub use memory_provenance_model::MemoryProvenance;
pub use memory_retrieval_error::MemoryRetrievalError;
pub use memory_retrieval_model::{
    DEFAULT_MEMORY_RETRIEVAL_BUDGET, DEFAULT_MEMORY_RETRIEVAL_COMMUNITIES,
    DEFAULT_MEMORY_RETRIEVAL_MAX_DEPTH, DEFAULT_MEMORY_RETRIEVAL_SEEDS,
    DEFAULT_MEMORY_RETRIEVAL_SUBCENTROIDS, MemoryRetrievalConfig, MemoryRetrievalHit,
    MemoryRetrievalIndex, MemoryRetrievalMode, MemoryRetrievalResult,
};
pub use memory_vector_error::MemoryVectorError;
pub use memory_vector_model::{
    MemoryVectorBuildResult, MemoryVectorId, MemoryVectorInfo, MemoryVectorLocation,
    MemoryVectorSet, MemoryVectorStats,
};
pub use migration::{MigrationError, MigrationResult, migrate_file};
pub use model_auth::ModelRequestAuth;
pub use model_switchboard::{
    EmbeddingModelEndpoint, GeneralModelEndpoint, ModelAuthKind, ModelCapability, ModelProvider,
    ModelReasoningEffort, ModelSwitchboard, ModelSwitchboardConfig,
};
pub use openai_codex_device_auth::{
    OPENAI_CODEX_AUTH_ISSUER, OPENAI_CODEX_DEVICE_LOGIN_TIMEOUT_SECS, OPENAI_CODEX_OAUTH_CLIENT_ID,
    OpenAiCodexDeviceAuth, OpenAiCodexDeviceAuthError, OpenAiCodexDeviceCode,
};
pub use openai_codex_general::{
    OPENAI_CODEX_COMPAT_VERSION, OPENAI_CODEX_RESPONSES_URL, OpenAiCodexGeneralEndpoint,
};
pub use openai_ready_embedding::{
    DEFAULT_REMOTE_EMBEDDING_BATCH_SIZE, DEFAULT_REMOTE_EMBEDDING_CONCURRENCY,
    OpenAiReadyEmbeddingEndpoint,
};
pub use openai_ready_general::OpenAiReadyGeneralEndpoint;
pub use packed_vector_error::PackedVectorError;
pub use packed_vector_model::{PackedVectorId, PackedVectorInfo, PackedVectorStats};
pub use phylactery::Phylactery;
pub use phylactery_error::PhylacteryError;
pub use project_history_model::{
    ProjectFileRef, ProjectRepositoryKind, ProjectRepositoryManagement, ProjectRepositoryRef,
    ProjectRevisionCorrelation, ProjectRevisionRef, RelSemanticCut,
};
pub use runtime_host::memory_search::{
    MAX_MEMORY_SEARCH_QUERY_BYTES, MemorySearchItem, MemorySearchLane, MemorySearchResult,
};
pub use runtime_host::status::RuntimeBackgroundStatus;
pub use runtime_host::{ReliquaryRuntimeHost, ReliquaryRuntimeHostError, ReliquaryRuntimeRoutes};
pub use search_error::SearchError;
pub use search_model::{
    DEFAULT_LEXICAL_WEIGHT, DEFAULT_SEARCH_CANDIDATE_LIMIT, DEFAULT_SEARCH_RESULT_LIMIT,
    DEFAULT_SEMANTIC_WEIGHT, RetrievalConfig, SearchCandidate,
};
pub use semantic_search_error::SemanticSearchError;
pub use semantic_search_model::{MAX_SEMANTIC_SEARCH_LIMIT, SemanticSearchHit};
pub use turn_ingest_model::{IncomingAttachment, IncomingTurn, IngestedTurn};
pub use vector_generation_error::VectorGenerationError;
pub use vector_generation_model::{VectorGeneration, VectorGenerationId, VectorGenerationStats};

#[cfg(test)]
mod archive_inventory_tests;
#[cfg(test)]
mod archive_search_tests;
#[cfg(test)]
mod archive_tests;
#[cfg(test)]
mod archive_vector_tests;
#[cfg(test)]
mod community_end_to_end_bench;
#[cfg(test)]
mod community_end_to_end_grid_bench;
#[cfg(test)]
mod community_lexical_escape_bench;
#[cfg(test)]
mod community_retrieval_bench;
#[cfg(test)]
mod community_routing_bench;
#[cfg(test)]
mod community_routing_bench_fixture;
#[cfg(test)]
mod community_routing_bench_support;
#[cfg(test)]
mod community_routing_cached_bench;
#[cfg(test)]
mod community_routing_holdout_bench;
#[cfg(test)]
mod community_routing_scale_bench;
#[cfg(test)]
mod community_routing_score_bench;
#[cfg(test)]
mod community_routing_tests;
#[cfg(test)]
mod community_scan_merge_bench;
#[cfg(test)]
mod community_scan_merge_tests;
#[cfg(test)]
mod community_subcentroid_routing;
#[cfg(test)]
mod community_test_support;
#[cfg(test)]
mod community_tests;
#[cfg(test)]
mod community_traversal_bench;
#[cfg(test)]
mod community_traversal_bench_fixture;
#[cfg(test)]
mod community_traversal_bench_support;
#[cfg(test)]
mod compatibility_profile_tests;
#[cfg(test)]
mod config_tests;
#[cfg(test)]
mod container_tests;
#[cfg(test)]
mod conversation_compaction_tests;
#[cfg(test)]
mod conversation_search_tests;
#[cfg(test)]
mod conversation_tests;
#[cfg(test)]
mod credential_tests;
#[cfg(test)]
mod cva_reconcile_conflict_tests;
#[cfg(test)]
mod cva_reconcile_derived_tests;
#[cfg(test)]
mod cva_reconcile_graph_policy_tests;
#[cfg(test)]
mod cva_reconcile_graph_test_support;
#[cfg(test)]
mod cva_reconcile_graph_tests;
#[cfg(test)]
mod cva_reconcile_grouped_tests;
#[cfg(test)]
mod cva_reconcile_guard_tests;
#[cfg(test)]
mod cva_reconcile_link_tests;
#[cfg(test)]
mod cva_reconcile_memory_tests;
#[cfg(test)]
mod cva_reconcile_merge_tests;
#[cfg(test)]
mod cva_reconcile_promotion_tests;
#[cfg(test)]
mod cva_reconcile_tests;
#[cfg(test)]
mod dream_candidate_test_support;
#[cfg(test)]
mod dream_candidate_tests;
#[cfg(test)]
mod dream_canonical_test_support;
#[cfg(test)]
mod dream_canonical_tests;
#[cfg(test)]
mod dream_classifier_tests;
#[cfg(test)]
mod dream_duplicate_tests;
#[cfg(test)]
mod dream_lifecycle_tests;
#[cfg(test)]
mod dream_processor_tests;
#[cfg(test)]
mod dream_publisher_tests;
#[cfg(test)]
mod dream_temporal_tests;
#[cfg(test)]
mod dream_verifier_tests;
#[cfg(test)]
mod echo_tests;
#[cfg(test)]
mod episode_tests;
#[cfg(test)]
mod file_memory_link_tests;
#[cfg(test)]
mod file_tests;
#[cfg(test)]
mod fragment_tests;
#[cfg(test)]
mod graph_tests;
#[cfg(test)]
mod history_tests;
#[cfg(test)]
mod interaction_background_tests;
#[cfg(test)]
mod interaction_runtime_tests;
#[cfg(test)]
mod interaction_session_tests;
#[cfg(test)]
mod lexical_index_tests;
#[cfg(test)]
mod master_key_tests;
#[cfg(test)]
mod memory_codec_tests;
#[cfg(test)]
mod memory_retrieval_stale_tests;
#[cfg(test)]
mod memory_retrieval_tests;
#[cfg(test)]
mod memory_tests;
#[cfg(test)]
mod memory_vector_tests;
#[cfg(test)]
mod migration_tests;
#[cfg(test)]
mod model_switchboard_tests;
#[cfg(test)]
mod openai_codex_device_auth_tests;
#[cfg(test)]
mod openai_codex_general_tests;
#[cfg(test)]
mod openai_ready_embedding_tests;
#[cfg(test)]
mod openai_ready_general_tests;
#[cfg(test)]
mod packed_vector_tests;
#[cfg(test)]
mod phylactery_dream_tests;
#[cfg(test)]
mod phylactery_tests;
#[cfg(test)]
mod project_file_attachment_tests;
#[cfg(test)]
mod project_history_reconcile_tests;
#[cfg(test)]
mod project_history_tests;
#[cfg(test)]
mod reliquary_tests;
#[cfg(test)]
mod runtime_host_backpressure_tests;
#[cfg(test)]
mod runtime_host_route_tests;
#[cfg(test)]
mod runtime_host_test_support;
#[cfg(test)]
mod runtime_host_tests;
#[cfg(test)]
mod runtime_host_vector_tests;
#[cfg(test)]
mod search_policy_tests;
#[cfg(test)]
mod search_tests;
#[cfg(test)]
mod semantic_search_tests;
#[cfg(test)]
mod turn_ingest_tests;
#[cfg(test)]
mod vector_generation_tests;
#[cfg(test)]
mod vector_generation_validation_tests;
