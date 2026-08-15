use crate::{ArchiveVectorId, EmbeddingProfileId};

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct VectorGenerationId(pub [u8; 32]);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct VectorGeneration {
    pub id: VectorGenerationId,
    pub profile_id: EmbeddingProfileId,
    pub archive_vector_id: ArchiveVectorId,
    pub source_archive_version: u64,
    pub global_version: u64,
    pub vector_version: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct VectorGenerationStats {
    pub generations: usize,
    pub active_profiles: usize,
    pub vector_version: u64,
}
