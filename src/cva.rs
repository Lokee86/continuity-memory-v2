use crate::archive_vector_store::ArchiveVectorStore;
use crate::community_store::CommunityStore;
use crate::compatibility_profile_store::CompatibilityProfileStore;
use crate::conversation_compaction_store::ConversationCompactionStore;
use crate::cva_memory_publish::publish_memory_parts;
use crate::dream_duplicate_index::DuplicateIndex;
use crate::graph_store::GraphStore;
use crate::insomnia::store::InsomniaStore;
use crate::interaction_stream_store::InteractionStreamStore;
use crate::lexical_index::LexicalIndex;
use crate::memory_store::MemoryStore;
use crate::memory_vector_store::MemoryVectorStore;
use crate::packed_vector_store::PackedVectorStore;
use crate::vector_generation_store::VectorGenerationStore;
use crate::{
    Archive, ArchiveError, ArchiveRecordVersion, ArchiveStats, Branch, Container,
    ConversationMetadata, CvaError, Episode, EpisodeBoundary, EpisodeBuildResult, EpisodeConfig,
    EpisodeId, EpisodeOrigin, FileId, Fragment, FragmentConfig, FragmentId, Memory, MemoryDraft,
    MemoryError, MemoryId, MemoryStats, Node, ResolvedTurn, StoredFile,
};

pub struct Cva {
    pub(crate) container: Container,
    pub(crate) archive: Archive,
    pub(crate) memories: MemoryStore,
    pub(crate) graph: GraphStore,
    pub(crate) communities: CommunityStore,
    pub(crate) duplicate_index: DuplicateIndex,
    pub(crate) insomnia: InsomniaStore,
    pub(crate) lexical_index: LexicalIndex,
    pub(crate) packed_vectors: PackedVectorStore,
    pub(crate) memory_vectors: MemoryVectorStore,
    pub(crate) archive_vectors: ArchiveVectorStore,
    pub(crate) compatibility_profiles: CompatibilityProfileStore,
    pub(crate) vector_generations: VectorGenerationStore,
    pub(crate) conversation_compactions: ConversationCompactionStore,
    pub(crate) interaction_streams: InteractionStreamStore,
}

impl Cva {
    pub fn scope_kind(&self) -> crate::ReliquaryScopeKind {
        self.container
            .identity()
            .and_then(|identity| identity.scope)
            .unwrap_or(crate::ReliquaryScopeKind::Project)
    }

    pub fn is_legacy_cva(&self) -> bool {
        self.container.identity().is_none()
    }

    pub fn owner_id(&self) -> Option<String> {
        self.container.owner_id()
    }

    pub fn owner_uuid(&self) -> Option<[u8; 16]> {
        self.container.owner_uuid()
    }

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

    pub fn set_conversation_title(
        &mut self,
        conversation_id: &str,
        title: String,
    ) -> Result<bool, ArchiveError> {
        let active = self
            .archive
            .conversation_metadata(conversation_id)
            .is_some_and(|metadata| metadata.active);
        self.archive.put_conversation_metadata(
            &mut self.container,
            ConversationMetadata {
                conversation_id: conversation_id.to_owned(),
                title: Some(title),
                active,
            },
        )
    }

    pub fn set_conversation_active(
        &mut self,
        conversation_id: &str,
        active: bool,
    ) -> Result<bool, ArchiveError> {
        let title = self
            .archive
            .conversation_metadata(conversation_id)
            .and_then(|metadata| metadata.title.clone());
        self.archive.put_conversation_metadata(
            &mut self.container,
            ConversationMetadata {
                conversation_id: conversation_id.to_owned(),
                title,
                active,
            },
        )
    }

    pub fn conversation_metadata(&self, conversation_id: &str) -> Option<&ConversationMetadata> {
        self.archive.conversation_metadata(conversation_id)
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
        publish_memory_parts(
            &self.archive,
            &mut self.memories,
            &mut self.container,
            id,
            expected_revision,
            draft,
        )
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

    pub fn memory_ids(&self) -> Vec<MemoryId> {
        self.memories.current_ids()
    }

    pub fn memory_version(&self) -> u64 {
        self.memories.memory_version()
    }

    pub fn memory_body_id(&self, id: MemoryId) -> Result<crate::MemoryBodyId, MemoryError> {
        self.memories.current_body_id(id)
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

    pub fn store_file(
        &mut self,
        filename: String,
        mime_type: Option<String>,
        bytes: &[u8],
    ) -> Result<StoredFile, ArchiveError> {
        self.archive
            .store_file(&mut self.container, filename, mime_type, bytes)
    }

    pub fn file(&self, id: FileId) -> Option<&StoredFile> {
        self.archive.file(id)
    }

    pub fn files(&self) -> Vec<StoredFile> {
        self.archive.files()
    }

    pub fn file_bytes(&mut self, id: FileId) -> Result<Vec<u8>, ArchiveError> {
        self.archive.file_bytes(&mut self.container, id)
    }

    pub fn conversation_compactions(
        &self,
        conversation_id: &str,
    ) -> Vec<crate::ConversationCompaction> {
        self.conversation_compactions
            .for_conversation(conversation_id)
    }

    pub fn put_conversation_compaction(
        &mut self,
        conversation_id: String,
        through_message_id: String,
        summary: String,
        supersedes_through_message_id: Option<&str>,
    ) -> Result<crate::ConversationCompaction, CvaError> {
        let record = self.conversation_compactions.put(
            &mut self.container,
            conversation_id,
            through_message_id,
            summary,
            supersedes_through_message_id,
        )?;
        Ok(record)
    }

    pub(crate) fn put_interaction_stream(
        &mut self,
        record: crate::InteractionStreamRecord,
    ) -> Result<(), CvaError> {
        self.interaction_streams.put(&mut self.container, record)
    }

    pub(crate) fn interaction_streams_for_session(
        &self,
        session_id: &str,
    ) -> Vec<crate::InteractionStreamRecord> {
        self.interaction_streams.records_for_session(session_id)
    }

    pub(crate) fn interaction_stream_records(&self) -> Vec<crate::InteractionStreamRecord> {
        self.interaction_streams.all_records()
    }

    pub fn sync(&self) -> Result<(), CvaError> {
        self.container.sync()?;
        Ok(())
    }
}
