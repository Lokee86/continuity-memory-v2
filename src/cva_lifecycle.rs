use crate::archive_rebuild::ArchiveOpenState;
use crate::archive_vector_rebuild::ArchiveVectorOpenState;
use crate::archive_vector_store::ArchiveVectorStore;
use crate::community_store::{CommunityOpenState, CommunityStore};
use crate::compatibility_profile_rebuild::CompatibilityProfileOpenState;
use crate::compatibility_profile_store::CompatibilityProfileStore;
use crate::conversation_compaction_store::{
    ConversationCompactionOpenState, ConversationCompactionStore,
};
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
use crate::{Archive, Container, Cva, CvaError};
use std::path::Path;

impl Cva {
    pub fn create(path: impl AsRef<Path>) -> Result<Self, CvaError> {
        Self::create_scope(path, crate::ReliquaryScopeKind::Project)
    }

    pub fn create_project(path: impl AsRef<Path>) -> Result<Self, CvaError> {
        Self::create_scope(path, crate::ReliquaryScopeKind::Project)
    }

    pub fn create_organization(path: impl AsRef<Path>) -> Result<Self, CvaError> {
        Self::create_scope(path, crate::ReliquaryScopeKind::Organization)
    }

    pub fn create_connection(path: impl AsRef<Path>) -> Result<Self, CvaError> {
        Self::create_scope(path, crate::ReliquaryScopeKind::Connection)
    }

    pub fn open_project(path: impl AsRef<Path>) -> Result<Self, CvaError> {
        Self::open_scope(path, crate::ReliquaryScopeKind::Project)
    }

    pub fn open_organization(path: impl AsRef<Path>) -> Result<Self, CvaError> {
        Self::open_scope(path, crate::ReliquaryScopeKind::Organization)
    }

    pub fn open_connection(path: impl AsRef<Path>) -> Result<Self, CvaError> {
        Self::open_scope(path, crate::ReliquaryScopeKind::Connection)
    }

    fn open_scope(
        path: impl AsRef<Path>,
        expected: crate::ReliquaryScopeKind,
    ) -> Result<Self, CvaError> {
        let cva = Self::open(path)?;
        if cva.scope_kind() != expected {
            return Err(CvaError::InvalidContainerIdentity(
                "Reliquary scope kind does not match the requested API",
            ));
        }
        Ok(cva)
    }

    pub(crate) fn create_scope(
        path: impl AsRef<Path>,
        scope: crate::ReliquaryScopeKind,
    ) -> Result<Self, CvaError> {
        let container = Container::create_with_identity(
            path,
            crate::ContainerIdentity {
                file_kind: crate::FileKind::Reliquary,
                scope: Some(scope),
            },
        )?;
        Self::initialize(container)
    }

    pub(crate) fn create_scope_with_uuid(
        path: impl AsRef<Path>,
        scope: crate::ReliquaryScopeKind,
        owner_uuid: [u8; 16],
    ) -> Result<Self, CvaError> {
        let container = Container::create_with_identity_and_uuid(
            path,
            crate::ContainerIdentity {
                file_kind: crate::FileKind::Reliquary,
                scope: Some(scope),
            },
            owner_uuid,
        )?;
        Self::initialize(container)
    }

    #[cfg(test)]
    pub(crate) fn create_legacy_cva(path: impl AsRef<Path>) -> Result<Self, CvaError> {
        Self::initialize(Container::create(path)?)
    }

    #[cfg(test)]
    pub(crate) fn create_legacy_typed(
        path: impl AsRef<Path>,
        scope: crate::ReliquaryScopeKind,
    ) -> Result<Self, CvaError> {
        Self::initialize(Container::create_with_legacy_identity(
            path,
            crate::ContainerIdentity {
                file_kind: crate::FileKind::Reliquary,
                scope: Some(scope),
            },
        )?)
    }

    fn initialize(mut container: Container) -> Result<Self, CvaError> {
        let archive = Archive::empty();
        let memories = MemoryStore::empty();
        let mut graph = GraphStore::empty();
        let communities = CommunityStore::default();
        let insomnia = InsomniaStore::empty();
        let lexical_index = LexicalIndex::default();
        let packed_vectors = PackedVectorStore::default();
        let memory_vectors = MemoryVectorStore::default();
        let archive_vectors = ArchiveVectorStore::default();
        let compatibility_profiles = CompatibilityProfileStore::default();
        let vector_generations = VectorGenerationStore::empty();
        let interaction_streams = InteractionStreamStore::default();
        let conversation_compactions = ConversationCompactionStore::empty();
        archive.initialize_history_format(&mut container)?;
        memories.initialize(&mut container)?;
        graph.initialize(&mut container)?;
        insomnia.initialize(&mut container)?;
        packed_vectors.initialize(&mut container)?;
        memory_vectors.initialize(&mut container)?;
        archive_vectors.initialize(&mut container)?;
        compatibility_profiles.initialize(&mut container)?;
        vector_generations.initialize(&mut container)?;
        container.sync()?;
        Ok(Self {
            container,
            archive,
            memories,
            duplicate_index: DuplicateIndex::empty(),
            graph,
            communities,
            insomnia,
            lexical_index,
            packed_vectors,
            memory_vectors,
            archive_vectors,
            compatibility_profiles,
            vector_generations,
            interaction_streams,
            conversation_compactions,
        })
    }

    pub fn open(path: impl AsRef<Path>) -> Result<Self, CvaError> {
        let mut archive_state = ArchiveOpenState::new();
        let mut memory_state = MemoryOpenState::new();
        let mut graph_state = GraphOpenState::new();
        let mut community_state = CommunityOpenState::new();
        let mut insomnia_state = InsomniaOpenState::new();
        let mut packed_state = PackedVectorOpenState::new();
        let mut memory_vector_state = MemoryVectorOpenState::new();
        let mut archive_vector_state = ArchiveVectorOpenState::new();
        let mut profile_state = CompatibilityProfileOpenState::new();
        let mut generation_state = VectorGenerationOpenState::new();
        let mut interaction_streams = InteractionStreamStore::default();
        let mut compaction_state = ConversationCompactionOpenState::default();
        let mut container = Container::open_scanned(path, |chunk, payload, latest_global| {
            archive_state.ingest(chunk, payload, latest_global)?;
            memory_state.ingest(chunk, payload, latest_global)?;
            graph_state.ingest(chunk, payload, latest_global)?;
            community_state.ingest(payload)?;
            insomnia_state.ingest(chunk, payload)?;
            packed_state.ingest(chunk, payload)?;
            memory_vector_state.ingest(chunk, payload)?;
            archive_vector_state.ingest(chunk, payload)?;
            profile_state.ingest(chunk, payload)?;
            generation_state.ingest(chunk, payload, latest_global)?;
            interaction_streams.ingest(payload)?;
            compaction_state.ingest(chunk, payload)?;
            Ok::<(), CvaError>(())
        })?;
        if let Some(identity) = container.identity()
            && identity.file_kind != crate::FileKind::Reliquary
        {
            return Err(CvaError::InvalidContainerIdentity(
                "file is not a Reliquary",
            ));
        }
        let archive = archive_state.finish()?;
        archive.validate_references()?;
        let memories = memory_state.finish()?;
        memories.validate_provenance(&archive)?;
        validate_file_memory_targets(&archive, &memories)?;
        let graph = graph_state.finish(&memories)?;
        let communities = community_state.finish(&graph, container.owner_uuid())?;
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
        let conversation_compactions = compaction_state.finish(&mut container)?;
        Ok(Self {
            container,
            archive,
            memories,
            duplicate_index: DuplicateIndex::empty(),
            graph,
            communities,
            insomnia,
            lexical_index,
            packed_vectors,
            memory_vectors,
            archive_vectors,
            compatibility_profiles,
            vector_generations,
            interaction_streams,
            conversation_compactions,
        })
    }
}
