use crate::archive_vector_store::ArchiveVectorStore;
use crate::compatibility_profile_store::CompatibilityProfileStore;
use crate::insomnia::store::InsomniaStore;
use crate::memory_store::MemoryStore;
use crate::packed_vector_store::PackedVectorStore;
use crate::vector_generation_store::VectorGenerationStore;
use crate::{
    Archive, ArchiveError, ArchiveRecordVersion, ArchiveStats, Branch, Container, CvaError,
    Episode, EpisodeBoundary, EpisodeBuildResult, EpisodeConfig, EpisodeId, EpisodeOrigin,
    Fragment, FragmentConfig, FragmentId, Memory, MemoryDraft, MemoryError, MemoryId, MemoryStats,
    Node, ResolvedTurn,
};

pub struct Cva {
    pub(crate) container: Container,
    pub(crate) archive: Archive,
    pub(crate) memories: MemoryStore,
    pub(crate) insomnia: InsomniaStore,
    pub(crate) packed_vectors: PackedVectorStore,
    pub(crate) archive_vectors: ArchiveVectorStore,
    pub(crate) compatibility_profiles: CompatibilityProfileStore,
    pub(crate) vector_generations: VectorGenerationStore,
}

impl Cva {
    pub fn archive(&self) -> &Archive {
        &self.archive
    }

    pub fn append_node(
        &mut self,
        id: String,
        conversation_id: String,
        parent_id: Option<String>,
        role: String,
        timestamp_ns: i64,
        content: &str,
    ) -> Result<Node, ArchiveError> {
        self.archive.append_node(
            &mut self.container,
            id,
            conversation_id,
            parent_id,
            role,
            timestamp_ns,
            content,
        )
    }

    pub fn append_branch(&mut self, branch: Branch) -> Result<(), ArchiveError> {
        self.archive.append_branch(&mut self.container, branch)
    }

    pub fn branch_turns(
        &mut self,
        conversation_id: &str,
        branch_id: &str,
    ) -> Result<Vec<ResolvedTurn>, ArchiveError> {
        self.archive
            .branch_turns(&mut self.container, conversation_id, branch_id)
    }

    pub fn branch_at(
        &mut self,
        conversation_id: &str,
        branch_id: &str,
        archive_version: u64,
    ) -> Result<Option<Branch>, ArchiveError> {
        self.archive.branch_at(
            &mut self.container,
            conversation_id,
            branch_id,
            archive_version,
        )
    }

    pub fn materialize_branch_fragments(
        &mut self,
        conversation_id: &str,
        branch_id: &str,
        config: FragmentConfig,
        close_tail: bool,
    ) -> Result<Vec<Fragment>, ArchiveError> {
        self.archive.materialize_branch_fragments(
            &mut self.container,
            conversation_id,
            branch_id,
            config,
            close_tail,
        )
    }

    pub fn materialize_path_fragments(
        &mut self,
        conversation_id: &str,
        leaf_node_id: &str,
        config: FragmentConfig,
        close_tail: bool,
    ) -> Result<Vec<Fragment>, ArchiveError> {
        self.archive.materialize_path_fragments(
            &mut self.container,
            conversation_id,
            leaf_node_id,
            config,
            close_tail,
        )
    }

    pub fn fragment_turns(&mut self, id: FragmentId) -> Result<Vec<ResolvedTurn>, ArchiveError> {
        self.archive.fragment_turns(&mut self.container, id)
    }

    pub fn fragment_text(&mut self, id: FragmentId) -> Result<String, ArchiveError> {
        self.archive.fragment_text(&mut self.container, id)
    }

    pub fn materialize_branch_episodes(
        &mut self,
        conversation_id: &str,
        branch_id: &str,
        config: EpisodeConfig,
        origin: EpisodeOrigin,
        close_tail: Option<(EpisodeBoundary, i64)>,
    ) -> Result<EpisodeBuildResult, ArchiveError> {
        self.archive.materialize_branch_episodes(
            &mut self.container,
            conversation_id,
            branch_id,
            config,
            origin,
            close_tail,
        )
    }

    pub fn materialize_path_episodes(
        &mut self,
        conversation_id: &str,
        leaf_node_id: &str,
        config: EpisodeConfig,
        origin: EpisodeOrigin,
        close_tail: Option<(EpisodeBoundary, i64)>,
    ) -> Result<EpisodeBuildResult, ArchiveError> {
        self.archive.materialize_path_episodes(
            &mut self.container,
            conversation_id,
            leaf_node_id,
            config,
            origin,
            close_tail,
        )
    }

    pub fn episodes(&self) -> Vec<Episode> {
        self.archive.episodes()
    }

    pub fn episodes_for_conversation(&self, conversation_id: &str) -> Vec<Episode> {
        self.archive.episodes_for_conversation(conversation_id)
    }

    pub fn episode(&self, id: EpisodeId) -> Option<&Episode> {
        self.archive.episode(id)
    }

    pub fn episode_turns(&mut self, id: EpisodeId) -> Result<Vec<ResolvedTurn>, ArchiveError> {
        self.archive.episode_turns(&mut self.container, id)
    }

    pub fn publish_memory(
        &mut self,
        id: Option<MemoryId>,
        expected_revision: u64,
        draft: MemoryDraft,
    ) -> Result<(Memory, bool), MemoryError> {
        self.validate_memory_provenance(&draft)?;
        self.memories
            .publish(&mut self.container, id, expected_revision, draft)
    }

    fn validate_memory_provenance(&self, draft: &MemoryDraft) -> Result<(), MemoryError> {
        match (draft.source_episode_id, draft.source_node_id.as_deref()) {
            (Some(episode_id), Some(node_id)) => {
                if !self
                    .archive
                    .episode_contains_node(episode_id, node_id)
                    .map_err(|_| MemoryError::InvalidProvenance)?
                {
                    return Err(MemoryError::InvalidProvenance);
                }
            }
            (None, None) => {}
            _ => return Err(MemoryError::InvalidProvenance),
        }
        match (
            draft.content_source_conversation_id.as_deref(),
            draft.content_source_node_id.as_deref(),
        ) {
            (Some(conversation_id), Some(node_id))
                if self.archive.has_node(conversation_id, node_id) => {}
            (None, None) => {}
            _ => return Err(MemoryError::InvalidProvenance),
        }
        Ok(())
    }

    pub fn memory(&mut self, id: MemoryId) -> Result<Memory, MemoryError> {
        self.memories.memory(&mut self.container, id)
    }

    pub fn memory_revision(&mut self, id: MemoryId, revision: u64) -> Result<Memory, MemoryError> {
        self.memories
            .memory_revision(&mut self.container, id, revision)
    }

    pub fn memory_stats(&self) -> MemoryStats {
        self.memories.stats()
    }

    pub fn memory_version(&self) -> u64 {
        self.memories.memory_version()
    }

    pub fn stats(&self) -> ArchiveStats {
        self.archive.stats()
    }

    pub fn archive_version(&self) -> u64 {
        self.archive.archive_version()
    }

    pub fn record_versions(&self) -> &[ArchiveRecordVersion] {
        self.archive.record_versions()
    }

    pub fn record_version(&self, archive_version: u64) -> Option<&ArchiveRecordVersion> {
        self.archive.record_version(archive_version)
    }

    pub fn branches(&self) -> Vec<Branch> {
        self.archive.branches()
    }

    pub fn fragments(&self) -> Vec<Fragment> {
        self.archive.fragments()
    }

    pub fn sync(&self) -> Result<(), CvaError> {
        self.container.sync()?;
        Ok(())
    }
}
