use crate::archive_rebuild::ArchiveOpenState;
use crate::archive_vector_rebuild::ArchiveVectorOpenState;
use crate::archive_vector_store::ArchiveVectorStore;
use crate::packed_vector_rebuild::PackedVectorOpenState;
use crate::packed_vector_store::PackedVectorStore;
use crate::{
    Archive, ArchiveError, ArchiveRecordVersion, ArchiveStats, Branch, Container, CvaError,
    Fragment, FragmentConfig, FragmentId, Node, ResolvedTurn,
};
use std::path::Path;

pub struct Cva {
    pub(crate) container: Container,
    pub(crate) archive: Archive,
    pub(crate) packed_vectors: PackedVectorStore,
    pub(crate) archive_vectors: ArchiveVectorStore,
}

impl Cva {
    pub fn create(path: impl AsRef<Path>) -> Result<Self, CvaError> {
        let mut container = Container::create(path)?;
        let archive = Archive::empty();
        let packed_vectors = PackedVectorStore::default();
        let archive_vectors = ArchiveVectorStore::default();
        archive.initialize_history_format(&mut container)?;
        packed_vectors.initialize(&mut container)?;
        archive_vectors.initialize(&mut container)?;
        container.sync()?;
        Ok(Self {
            container,
            archive,
            packed_vectors,
            archive_vectors,
        })
    }

    pub fn open(path: impl AsRef<Path>) -> Result<Self, CvaError> {
        let mut archive_state = ArchiveOpenState::new();
        let mut packed_state = PackedVectorOpenState::new();
        let mut archive_vector_state = ArchiveVectorOpenState::new();
        let container = Container::open_scanned(path, |chunk, payload, latest_global| {
            archive_state
                .ingest(chunk, payload, latest_global)
                .map_err(CvaError::from)?;
            packed_state
                .ingest(chunk, payload)
                .map_err(CvaError::from)?;
            archive_vector_state
                .ingest(chunk, payload)
                .map_err(CvaError::from)
        })?;
        let archive = archive_state.finish()?;
        archive.validate_references()?;
        let packed_vectors = packed_state.finish()?;
        let archive_vectors = archive_vector_state.finish(&archive, &packed_vectors)?;
        Ok(Self {
            container,
            archive,
            packed_vectors,
            archive_vectors,
        })
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

    pub fn fragments(&self) -> Vec<Fragment> {
        self.archive.fragments()
    }

    pub fn sync(&self) -> Result<(), CvaError> {
        self.container.sync()?;
        Ok(())
    }
}
