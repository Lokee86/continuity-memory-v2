use crate::archive_rebuild::ArchiveOpenState;
use crate::archive_vector_rebuild::ArchiveVectorOpenState;
use crate::archive_vector_store::ArchiveVectorStore;
use crate::cva_global_validation::validate_semantic_global_versions;
use crate::embedding_profile_rebuild::EmbeddingProfileOpenState;
use crate::embedding_profile_store::EmbeddingProfileStore;
use crate::packed_vector_rebuild::PackedVectorOpenState;
use crate::packed_vector_store::PackedVectorStore;
use crate::vector_generation_rebuild::VectorGenerationOpenState;
use crate::vector_generation_store::VectorGenerationStore;
use crate::{Archive, Container, Cva, CvaError};
use std::path::Path;

impl Cva {
    pub fn create(path: impl AsRef<Path>) -> Result<Self, CvaError> {
        let mut container = Container::create(path)?;
        let archive = Archive::empty();
        let packed_vectors = PackedVectorStore::default();
        let archive_vectors = ArchiveVectorStore::default();
        let embedding_profiles = EmbeddingProfileStore::default();
        let vector_generations = VectorGenerationStore::empty();
        archive.initialize_history_format(&mut container)?;
        packed_vectors.initialize(&mut container)?;
        archive_vectors.initialize(&mut container)?;
        embedding_profiles.initialize(&mut container)?;
        vector_generations.initialize(&mut container)?;
        container.sync()?;
        Ok(Self {
            container,
            archive,
            packed_vectors,
            archive_vectors,
            embedding_profiles,
            vector_generations,
        })
    }

    pub fn open(path: impl AsRef<Path>) -> Result<Self, CvaError> {
        let mut archive_state = ArchiveOpenState::new();
        let mut packed_state = PackedVectorOpenState::new();
        let mut archive_vector_state = ArchiveVectorOpenState::new();
        let mut profile_state = EmbeddingProfileOpenState::new();
        let mut generation_state = VectorGenerationOpenState::new();
        let container = Container::open_scanned(path, |chunk, payload, latest_global| {
            archive_state.ingest(chunk, payload, latest_global)?;
            packed_state.ingest(chunk, payload)?;
            archive_vector_state.ingest(chunk, payload)?;
            profile_state.ingest(chunk, payload)?;
            generation_state.ingest(chunk, payload, latest_global)?;
            Ok::<(), CvaError>(())
        })?;
        let archive = archive_state.finish()?;
        archive.validate_references()?;
        let packed_vectors = packed_state.finish()?;
        let archive_vectors = archive_vector_state.finish(&archive, &packed_vectors)?;
        let embedding_profiles = profile_state.finish()?;
        let vector_generations = generation_state.finish(
            &archive,
            &packed_vectors,
            &archive_vectors,
            &embedding_profiles,
        )?;
        validate_semantic_global_versions(&archive, &vector_generations)?;
        Ok(Self {
            container,
            archive,
            packed_vectors,
            archive_vectors,
            embedding_profiles,
            vector_generations,
        })
    }
}
