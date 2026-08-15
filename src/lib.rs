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
pub mod container;
mod container_error;
mod container_version;
pub mod cva;
mod cva_archive_vectors;
mod cva_embedding_profiles;
mod cva_error;
mod cva_global_validation;
mod cva_lifecycle;
mod cva_packed_vectors;
mod cva_vector_generations;
mod embedding_endpoint;
mod embedding_profile_codec;
mod embedding_profile_error;
mod embedding_profile_model;
mod embedding_profile_probe;
mod embedding_profile_rebuild;
mod embedding_profile_store;
mod fragment_model;
mod fragment_store;
mod fragmenter;
mod packed_vector_codec;
mod packed_vector_error;
mod packed_vector_model;
mod packed_vector_rebuild;
mod packed_vector_store;
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
pub use container::{ChunkRef, Container, ContainerError, FormatVersion};
pub use cva::Cva;
pub use cva_error::CvaError;
pub use embedding_endpoint::{
    EmbeddingEndpoint, EmbeddingEndpointDescriptor, EmbeddingEndpointError, EmbeddingMode,
    SimulatedEmbeddingEndpoint, VectorNormalization,
};
pub use embedding_profile_error::EmbeddingProfileError;
pub use embedding_profile_model::{
    EMBEDDING_PROBE_SUITE_VERSION, EmbeddingProfile, EmbeddingProfileId, EmbeddingProfileStats,
};
pub use fragment_model::{Fragment, FragmentConfig, FragmentId};
pub use lodestone_packed::{PackedVectors, ScalarType, VectorSchema};
pub use packed_vector_error::PackedVectorError;
pub use packed_vector_model::{PackedVectorId, PackedVectorInfo, PackedVectorStats};
pub use vector_generation_error::VectorGenerationError;
pub use vector_generation_model::{VectorGeneration, VectorGenerationId, VectorGenerationStats};

#[cfg(test)]
mod archive_tests;
#[cfg(test)]
mod archive_vector_tests;
#[cfg(test)]
mod container_tests;
#[cfg(test)]
mod embedding_profile_tests;
#[cfg(test)]
mod fragment_tests;
#[cfg(test)]
mod history_tests;
#[cfg(test)]
mod packed_vector_tests;
#[cfg(test)]
mod vector_generation_tests;
#[cfg(test)]
mod vector_generation_validation_tests;
