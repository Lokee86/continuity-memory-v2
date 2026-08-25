pub mod archive;
mod archive_codec;
mod archive_conversation;
mod archive_error;
mod archive_history;
mod archive_history_codec;
mod archive_history_model;
mod archive_lookup;
mod archive_model;
mod archive_object_index;
mod archive_rebuild;
mod archive_record_index;
mod archive_store;
mod archive_vector_codec;
mod archive_vector_error;
mod archive_vector_model;
mod archive_vector_rebuild;
mod archive_vector_store;
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
pub mod container;
mod container_error;
mod container_version;
mod credential;
mod credential_codec;
mod credential_crypto;
pub mod cva;
mod cva_archive_vectors;
mod cva_compatibility_profiles;
mod cva_conversation;
mod cva_error;
mod cva_file_memory;
mod cva_global_validation;
mod cva_graph;
mod cva_lifecycle;
mod cva_memory_publish;
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
mod cva_workspace;
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
mod memory_error;
mod memory_model;
mod memory_rebuild;
mod memory_store;
mod memory_vector_codec;
mod memory_vector_error;
mod memory_vector_model;
mod memory_vector_rebuild;
mod memory_vector_store;
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
mod workspace_metadata;
mod workspace_metadata_codec;
mod workspace_metadata_rebuild;
mod workspace_metadata_store;

pub use archive::Archive;
pub use archive_error::ArchiveError;
pub use archive_history_model::ArchiveRecordVersion;
pub use archive_model::{ArchiveStats, Branch, ContentId, ConversationSummary, Node, ResolvedTurn};
pub use archive_vector_error::ArchiveVectorError;
pub use archive_vector_model::{
    ArchiveVectorId, ArchiveVectorInfo, ArchiveVectorSet, ArchiveVectorStats,
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
pub use container::{ChunkRef, Container, ContainerError, FormatVersion};
pub use credential::{Credential, CredentialError, CredentialId, CredentialsConfig, SecretString};
pub use cva::Cva;
pub use cva_error::CvaError;
pub use cva_reconcile::{CvaComparison, CvaReconcileResult, CvaRelation};
pub use cva_reconcile_conflict::CvaReconcileConflict;
pub use cva_reconcile_error::CvaReconcileError;
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
pub use fragment_model::{Fragment, FragmentConfig, FragmentId};
pub use general_endpoint::{GeneralEndpoint, GeneralEndpointError, SimulatedGeneralEndpoint};
pub use graph_error::GraphError;
pub(crate) use graph_model::GraphNodeRecord;
pub use graph_model::{
    GraphDirection, GraphNeighbor, GraphRelation, GraphRelationChange, GraphRelationKind,
    GraphStats, MemoryGraphPath,
};
pub use insomnia::{
    DEFAULT_INSOMNIA_LEASE_NS, DEFAULT_INSOMNIA_MAX_ATTEMPTS, DEFAULT_INSOMNIA_POLL_NS,
    DEFAULT_INSOMNIA_RETRY_DELAY_NS, DEFAULT_INSOMNIA_WORKERS, EpisodeSchedulingResult,
    INSOMNIA_EXTRACTOR_CONTRACT_VERSION, INSOMNIA_SYSTEM_PROMPT, InsomniaAttempt,
    InsomniaCandidate, InsomniaDrainResult, InsomniaError, InsomniaEvidenceResult,
    InsomniaEvidenceTurn, InsomniaExtraction, InsomniaExtractionError, InsomniaExtractor,
    InsomniaLeaseToken, InsomniaPriority, InsomniaProcessError, InsomniaProcessResult,
    InsomniaRejection, InsomniaStats, InsomniaWork, InsomniaWorkState, InsomniaWorkerConfig,
    InsomniaWorkerError, MAX_INSOMNIA_CANDIDATES, MAX_INSOMNIA_EVIDENCE_BYTES,
    MAX_INSOMNIA_EVIDENCE_REQUESTS, MAX_INSOMNIA_EVIDENCE_TURNS, MAX_INSOMNIA_WORKERS,
    insomnia_schema,
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
    Memory, MemoryBodyId, MemoryDraft, MemoryId, MemoryRevisionId, MemoryStats,
};
pub use memory_vector_error::MemoryVectorError;
pub use memory_vector_model::{
    MemoryVectorBuildResult, MemoryVectorId, MemoryVectorInfo, MemoryVectorLocation,
    MemoryVectorSet, MemoryVectorStats,
};
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
pub use workspace_metadata::{
    MAX_WORKSPACE_ID_BYTES, MAX_WORKSPACE_NAME_BYTES, MAX_WORKSPACE_TYPE_BYTES, WorkspaceMetadata,
    WorkspaceMetadataError,
};

#[cfg(test)]
mod archive_inventory_tests;
#[cfg(test)]
mod archive_tests;
#[cfg(test)]
mod archive_vector_tests;
#[cfg(test)]
mod compatibility_profile_tests;
#[cfg(test)]
mod config_tests;
#[cfg(test)]
mod container_tests;
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
mod interaction_runtime_tests;
#[cfg(test)]
mod interaction_session_tests;
#[cfg(test)]
mod lexical_index_tests;
#[cfg(test)]
mod master_key_tests;
#[cfg(test)]
mod memory_tests;
#[cfg(test)]
mod memory_vector_tests;
#[cfg(test)]
mod model_switchboard_tests;
#[cfg(test)]
mod openai_codex_device_auth_tests;
#[cfg(test)]
mod openai_codex_general_tests;
#[cfg(test)]
mod openai_ready_embedding_tests;
#[cfg(test)]
mod packed_vector_tests;
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
#[cfg(test)]
mod workspace_metadata_tests;
