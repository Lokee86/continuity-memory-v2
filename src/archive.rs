use crate::archive_codec::{encode_branch, encode_node};
use crate::archive_object_index::{ContentIndex, FragmentIndex};
use crate::archive_profile::{profile_enabled, report_open};
use crate::archive_rebuild::ArchiveOpenState;
use crate::archive_record_index::{BranchIndex, NodeIndex};
use crate::archive_store::{hash_content, validate_text};
use crate::{
    ArchiveError, ArchiveRecordVersion, ArchiveStats, Branch, Container, Node, ResolvedTurn,
};
use std::path::Path;
use std::time::Instant;

pub struct Archive {
    pub(crate) container: Container,
    pub(crate) contents: ContentIndex,
    pub(crate) nodes: NodeIndex,
    pub(crate) branches: BranchIndex,
    pub(crate) fragments: FragmentIndex,
    pub(crate) record_versions: Vec<ArchiveRecordVersion>,
    pub(crate) next_archive_version: u64,
}

impl Archive {
    pub fn create(path: impl AsRef<Path>) -> Result<Self, ArchiveError> {
        let mut archive = Self::empty(Container::create(path)?);
        archive.initialize_history_format()?;
        Ok(archive)
    }

    pub fn open(path: impl AsRef<Path>) -> Result<Self, ArchiveError> {
        let profile = profile_enabled();
        let total_start = profile.then(Instant::now);
        let scan_start = profile.then(Instant::now);
        let mut state = ArchiveOpenState::new();
        let container = Container::open_scanned(path, |chunk, payload, latest_global| {
            state.ingest(chunk, payload, latest_global)
        })?;
        let scan_elapsed = scan_start.map(|start| start.elapsed());
        let archive = state.finish(container)?;
        let validate_start = profile.then(Instant::now);
        archive.validate_references()?;
        let validate_elapsed = validate_start.map(|start| start.elapsed());
        if let (Some(total), Some(scan), Some(validate)) = (
            total_start.map(|start| start.elapsed()),
            scan_elapsed,
            validate_elapsed,
        ) {
            report_open(&archive, total, scan, validate);
        }
        Ok(archive)
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
        validate_text(&id, "node id")?;
        validate_text(&conversation_id, "conversation id")?;
        validate_text(&role, "role")?;
        self.validate_parent(&conversation_id, parent_id.as_deref())?;

        let content_id = hash_content(content.as_bytes());
        let node = Node {
            id,
            conversation_id,
            parent_id,
            role,
            timestamp_ns,
            content_id,
        };
        if let Some(existing) = self.nodes.get(&node.conversation_id, &node.id) {
            return if existing == &node {
                Ok(existing.clone())
            } else {
                Err(ArchiveError::ConflictingNode)
            };
        }

        self.put_content(content_id, content)?;
        let record = self.container.append(&encode_node(&node)?)?;
        self.publish_record(record)?;
        self.nodes.insert(node.clone())?;
        Ok(node)
    }

    pub fn append_branch(&mut self, branch: Branch) -> Result<(), ArchiveError> {
        validate_text(&branch.id, "branch id")?;
        validate_text(&branch.conversation_id, "conversation id")?;
        self.require_node(
            &branch.conversation_id,
            &branch.leaf_node_id,
            ArchiveError::MissingLeaf,
        )?;
        if self.branches.get(&branch.conversation_id, &branch.id) == Some(&branch) {
            return Ok(());
        }
        if let Some(existing) = self.branches.get(&branch.conversation_id, &branch.id) {
            let path = self.branch_nodes(&branch.conversation_id, &branch.leaf_node_id)?;
            if !path.iter().any(|node| node.id == existing.leaf_node_id) {
                return Err(ArchiveError::InvalidBranchRevision);
            }
        }
        let record = self.container.append(&encode_branch(&branch)?)?;
        self.publish_record(record)?;
        self.branches.put(branch);
        Ok(())
    }

    pub fn branch_turns(
        &mut self,
        conversation_id: &str,
        branch_id: &str,
    ) -> Result<Vec<ResolvedTurn>, ArchiveError> {
        let branch = self
            .branches
            .get(conversation_id, branch_id)
            .ok_or(ArchiveError::MissingBranch)?
            .clone();
        let nodes = self.branch_nodes(conversation_id, &branch.leaf_node_id)?;
        nodes
            .into_iter()
            .map(|node| {
                let content = self.content(node.content_id)?;
                Ok(ResolvedTurn {
                    node_id: node.id,
                    role: node.role,
                    timestamp_ns: node.timestamp_ns,
                    content,
                })
            })
            .collect()
    }

    pub fn stats(&self) -> ArchiveStats {
        ArchiveStats {
            content_objects: self.contents.len(),
            nodes: self.nodes.len(),
            branches: self.branches.len(),
            fragments: self.fragments.len(),
        }
    }

    pub fn sync(&self) -> Result<(), ArchiveError> {
        Ok(self.container.sync()?)
    }
}
