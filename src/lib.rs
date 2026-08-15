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
mod cva_error;
mod cva_packed_vectors;
mod fragment_model;
mod fragment_store;
mod fragmenter;
mod packed_vector_codec;
mod packed_vector_error;
mod packed_vector_model;
mod packed_vector_rebuild;
mod packed_vector_store;

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
pub use fragment_model::{Fragment, FragmentConfig, FragmentId};
pub use lodestone_packed::{PackedVectors, ScalarType, VectorSchema};
pub use packed_vector_error::PackedVectorError;
pub use packed_vector_model::{PackedVectorId, PackedVectorInfo, PackedVectorStats};

#[cfg(test)]
mod archive_tests;
#[cfg(test)]
mod archive_vector_tests;
#[cfg(test)]
mod container_tests;
#[cfg(test)]
mod fragment_tests;
#[cfg(test)]
mod history_tests;
#[cfg(test)]
mod packed_vector_tests;
