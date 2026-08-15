pub mod archive;
mod archive_codec;
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
pub mod container;
mod container_error;
mod container_version;
mod credential;
mod credential_codec;
mod credential_crypto;
pub mod cva;
mod cva_archive_vectors;
mod cva_compatibility_profiles;
mod cva_error;
mod cva_global_validation;
mod cva_lifecycle;
mod cva_packed_vectors;
mod cva_vector_generations;
mod embedding_endpoint;
mod fragment_model;
mod fragment_store;
mod fragmenter;
mod lexical_search;
mod master_key;
mod master_key_entropy;
mod model_auth;
mod model_switchboard;
mod model_switchboard_codec;
mod openai_ready_embedding;
mod openai_ready_embedding_response;
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
mod vector_generation_codec;
mod vector_generation_error;
mod vector_generation_model;
mod vector_generation_rebuild;
mod vector_generation_store;
mod vector_generation_validation;

pub use archive::Archive;
pub use archive_error::ArchiveError;
pub use archive_history_model::ArchiveRecordVersion;
pub use archive_model::{ArchiveStats, Branch, ContentId, Node, ResolvedTurn};
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
pub use config::ContinuityConfig;
pub use config_error::ConfigError;
pub use container::{ChunkRef, Container, ContainerError, FormatVersion};
pub use credential::{Credential, CredentialError, CredentialId, CredentialsConfig, SecretString};
pub use cva::Cva;
pub use cva_error::CvaError;
pub use embedding_endpoint::{
    EmbeddingEndpoint, EmbeddingEndpointError, EmbeddingMode, SimulatedEmbeddingEndpoint,
    VectorNormalization,
};
pub use fragment_model::{Fragment, FragmentConfig, FragmentId};
pub use lodestone_packed::{PackedVectors, ScalarType, VectorSchema};
pub use master_key::{
    JsonMasterKeyStore, MASTER_KEY_BYTES, MasterKey, MasterKeyError, MasterKeyStore,
};
pub use model_auth::ModelRequestAuth;
pub use model_switchboard::{
    EmbeddingModelEndpoint, GeneralModelEndpoint, ModelAuthKind, ModelCapability, ModelProvider,
    ModelSwitchboard, ModelSwitchboardConfig,
};
pub use openai_ready_embedding::{
    DEFAULT_REMOTE_EMBEDDING_BATCH_SIZE, DEFAULT_REMOTE_EMBEDDING_CONCURRENCY,
    OpenAiReadyEmbeddingEndpoint,
};
pub use packed_vector_error::PackedVectorError;
pub use packed_vector_model::{PackedVectorId, PackedVectorInfo, PackedVectorStats};
pub use search_error::SearchError;
pub use search_model::{
    DEFAULT_LEXICAL_WEIGHT, DEFAULT_SEARCH_CANDIDATE_LIMIT, DEFAULT_SEARCH_RESULT_LIMIT,
    DEFAULT_SEMANTIC_WEIGHT, RetrievalConfig, SearchCandidate,
};
pub use semantic_search_error::SemanticSearchError;
pub use semantic_search_model::{MAX_SEMANTIC_SEARCH_LIMIT, SemanticSearchHit};
pub use vector_generation_error::VectorGenerationError;
pub use vector_generation_model::{VectorGeneration, VectorGenerationId, VectorGenerationStats};

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
mod credential_tests;
#[cfg(test)]
mod fragment_tests;
#[cfg(test)]
mod history_tests;
#[cfg(test)]
mod master_key_tests;
#[cfg(test)]
mod model_switchboard_tests;
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
mod vector_generation_tests;
#[cfg(test)]
mod vector_generation_validation_tests;
