use crate::archive_rebuild::ArchiveOpenState;
use crate::archive_vector_rebuild::ArchiveVectorOpenState;
use crate::archive_vector_store::ArchiveVectorStore;
use crate::compatibility_profile_rebuild::CompatibilityProfileOpenState;
use crate::compatibility_profile_store::CompatibilityProfileStore;
use crate::cva_global_validation::validate_semantic_global_versions;
use crate::dream_duplicate_index::DuplicateIndex;
use crate::file_memory_link_store::validate_file_memory_targets;
use crate::graph_rebuild::GraphOpenState;
use crate::graph_store::GraphStore;
use crate::insomnia::rebuild::InsomniaOpenState;
use crate::insomnia::store::InsomniaStore;
use crate::interaction_stream_store::InteractionStreamStore;
use crate::lexical_index::LexicalIndex;
use crate::memory_rebuild::MemoryOpenState;
use crate::memory_store::MemoryStore;
use crate::memory_vector_rebuild::MemoryVectorOpenState;
use crate::memory_vector_store::MemoryVectorStore;
use crate::packed_vector_rebuild::PackedVectorOpenState;
use crate::packed_vector_store::PackedVectorStore;
use crate::vector_generation_rebuild::VectorGenerationOpenState;
use crate::vector_generation_store::VectorGenerationStore;
use crate::workspace_metadata_rebuild::WorkspaceMetadataOpenState;
use crate::workspace_metadata_store::WorkspaceMetadataStore;
use crate::{Archive, Container, Cva, CvaError};
use std::path::Path;

impl Cva {
    pub fn create(path: impl AsRef<Path>) -> Result<Self, CvaError> {
        let mut container = Container::create(path)?;
        let archive = Archive::empty();
        let memories = MemoryStore::empty();
        let mut graph = GraphStore::empty();
        let insomnia = InsomniaStore::empty();
        let lexical_index = LexicalIndex::default();
        let packed_vectors = PackedVectorStore::default();
        let memory_vectors = MemoryVectorStore::default();
        let archive_vectors = ArchiveVectorStore::default();
        let compatibility_profiles = CompatibilityProfileStore::default();
        let vector_generations = VectorGenerationStore::empty();
        let mut workspace_metadata = WorkspaceMetadataStore::empty();
        let interaction_streams = InteractionStreamStore::default();
        archive.initialize_history_format(&mut container)?;
        memories.initialize(&mut container)?;
        graph.initialize(&mut container)?;
        insomnia.initialize(&mut container)?;
        packed_vectors.initialize(&mut container)?;
        memory_vectors.initialize(&mut container)?;
        archive_vectors.initialize(&mut container)?;
        compatibility_profiles.initialize(&mut container)?;
        vector_generations.initialize(&mut container)?;
        workspace_metadata.initialize_format(&mut container)?;
        container.sync()?;
        Ok(Self {
            container,
            archive,
            memories,
            duplicate_index: DuplicateIndex::empty(),
            graph,
            insomnia,
            lexical_index,
            packed_vectors,
            memory_vectors,
            archive_vectors,
            compatibility_profiles,
            vector_generations,
            workspace_metadata,
            interaction_streams,
        })
    }

    pub fn open(path: impl AsRef<Path>) -> Result<Self, CvaError> {
        let mut archive_state = ArchiveOpenState::new();
        let mut memory_state = MemoryOpenState::new();
        let mut graph_state = GraphOpenState::new();
        let mut insomnia_state = InsomniaOpenState::new();
        let mut packed_state = PackedVectorOpenState::new();
        let mut memory_vector_state = MemoryVectorOpenState::new();
        let mut archive_vector_state = ArchiveVectorOpenState::new();
        let mut profile_state = CompatibilityProfileOpenState::new();
        let mut generation_state = VectorGenerationOpenState::new();
        let mut workspace_state = WorkspaceMetadataOpenState::new();
        let mut interaction_streams = InteractionStreamStore::default();
        let container = Container::open_scanned(path, |chunk, payload, latest_global| {
            archive_state.ingest(chunk, payload, latest_global)?;
            memory_state.ingest(chunk, payload, latest_global)?;
            graph_state.ingest(chunk, payload, latest_global)?;
            insomnia_state.ingest(chunk, payload)?;
            packed_state.ingest(chunk, payload)?;
            memory_vector_state.ingest(chunk, payload)?;
            archive_vector_state.ingest(chunk, payload)?;
            profile_state.ingest(chunk, payload)?;
            generation_state.ingest(chunk, payload, latest_global)?;
            workspace_state.ingest(payload)?;
            interaction_streams.ingest(payload)?;
            Ok::<(), CvaError>(())
        })?;
        let archive = archive_state.finish()?;
        archive.validate_references()?;
        let memories = memory_state.finish()?;
        memories.validate_provenance(&archive)?;
        validate_file_memory_targets(&archive, &memories)?;
        let graph = graph_state.finish(&memories)?;
        let mut insomnia = insomnia_state.finish()?;
        insomnia.validate(&archive, &memories)?;
        insomnia.rebuild_schedule(&archive)?;
        let lexical_index = LexicalIndex::default();
        let packed_vectors = packed_state.finish()?;
        let compatibility_profiles = profile_state.finish()?;
        let memory_vectors =
            memory_vector_state.finish(&memories, &compatibility_profiles, &packed_vectors)?;
        let archive_vectors = archive_vector_state.finish(&archive, &packed_vectors)?;
        let vector_generations = generation_state.finish(
            &archive,
            &packed_vectors,
            &archive_vectors,
            &compatibility_profiles,
        )?;
        validate_semantic_global_versions(&archive, &memories, &graph, &vector_generations)?;
        let workspace_metadata = workspace_state.finish();
        Ok(Self {
            container,
            archive,
            memories,
            duplicate_index: DuplicateIndex::empty(),
            graph,
            insomnia,
            lexical_index,
            packed_vectors,
            memory_vectors,
            archive_vectors,
            compatibility_profiles,
            vector_generations,
            workspace_metadata,
            interaction_streams,
        })
    }
}
