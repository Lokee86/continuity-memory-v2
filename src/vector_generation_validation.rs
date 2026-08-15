use crate::archive_vector_store::ArchiveVectorStore;
use crate::embedding_profile_store::EmbeddingProfileStore;
use crate::packed_vector_store::PackedVectorStore;
use crate::{
    Archive, ArchiveVectorId, EmbeddingProfileId, VectorGeneration, VectorGenerationError,
    VectorGenerationId,
};
use sha2::{Digest, Sha256};

pub(crate) fn validate_generation_reference(
    archive: &Archive,
    packed_vectors: &PackedVectorStore,
    archive_vectors: &ArchiveVectorStore,
    profiles: &EmbeddingProfileStore,
    generation: &VectorGeneration,
) -> Result<(), VectorGenerationError> {
    if generation.source_archive_version == 0
        || generation.source_archive_version > archive.archive_version()
    {
        return Err(VectorGenerationError::SourceArchiveVersion);
    }
    let profile = profiles
        .get(generation.profile_id)
        .ok_or(VectorGenerationError::MissingProfile)?;
    let archive_vectors = archive_vectors
        .info(generation.archive_vector_id)
        .ok_or(VectorGenerationError::MissingArchiveVectors)?;
    if generation.source_archive_version < archive_vectors.max_fragment_archive_version {
        return Err(VectorGenerationError::SourceArchiveVersion);
    }
    let packed = packed_vectors
        .info(archive_vectors.packed_vector_id)
        .ok_or(VectorGenerationError::MissingArchiveVectors)?;
    if profile.dimensions != packed.schema.dimensions {
        return Err(VectorGenerationError::DimensionMismatch);
    }
    Ok(())
}

pub(crate) fn vector_generation_id(
    profile_id: EmbeddingProfileId,
    archive_vector_id: ArchiveVectorId,
    source_archive_version: u64,
) -> VectorGenerationId {
    let mut hash = Sha256::new();
    hash.update(b"CVA-VECTOR-GENERATION-V1\0");
    hash.update(profile_id.0);
    hash.update(archive_vector_id.0);
    hash.update(source_archive_version.to_le_bytes());
    VectorGenerationId(hash.finalize().into())
}
