use crate::archive_codec::{ArchiveRecord, decode_record, encode_content};
use crate::archive_history_codec::{decode_archive_format, decode_record_version};
use crate::archive_profile::{profile_enabled, report_rebuild};
use crate::{Archive, ArchiveError, Branch, ContentId, Node};
use sha2::{Digest, Sha256};
use std::collections::HashSet;
use std::time::Instant;

impl Archive {
    pub(crate) fn empty(container: crate::Container) -> Self {
        Self {
            container,
            contents: Default::default(),
            nodes: Default::default(),
            branches: Default::default(),
            fragments: Default::default(),
            record_versions: Vec::new(),
            next_archive_version: 1,
        }
    }

    pub(crate) fn put_content(&mut self, id: ContentId, content: &str) -> Result<(), ArchiveError> {
        if self.contents.contains(id) {
            if self.content(id)? != content {
                return Err(ArchiveError::HashCollision);
            }
            return Ok(());
        }
        let chunk = self
            .container
            .append(&encode_content(id, content.as_bytes())?)?;
        self.contents.insert(id, chunk);
        Ok(())
    }

    pub(crate) fn content(&mut self, id: ContentId) -> Result<String, ArchiveError> {
        let chunk = self.contents.get(id).ok_or(ArchiveError::MissingContent)?;
        match decode_record(&self.container.read(chunk)?)? {
            ArchiveRecord::Content(found, bytes) if found == id => {
                String::from_utf8(bytes).map_err(|_| ArchiveError::InvalidUtf8)
            }
            _ => Err(ArchiveError::MissingContent),
        }
    }

    pub(crate) fn branch_nodes(
        &self,
        conversation_id: &str,
        leaf_id: &str,
    ) -> Result<Vec<Node>, ArchiveError> {
        let mut nodes = Vec::new();
        let mut seen = HashSet::new();
        let mut current = Some(leaf_id.to_owned());
        while let Some(id) = current {
            if !seen.insert(id.clone()) {
                return Err(ArchiveError::NodeCycle);
            }
            let node = self
                .require_node(conversation_id, &id, ArchiveError::MissingNode)?
                .clone();
            current = node.parent_id.clone();
            nodes.push(node);
        }
        nodes.reverse();
        Ok(nodes)
    }

    pub(crate) fn rebuild_index(&mut self) -> Result<(), ArchiveError> {
        let profile = profile_enabled();
        let scan_start = profile.then(Instant::now);
        let mut format_seen = false;
        for chunk in self.container.chunks()? {
            let payload = self.container.read(chunk)?;
            if decode_archive_format(&payload)? {
                if format_seen {
                    return Err(ArchiveError::ConflictingArchiveFormat);
                }
                format_seen = true;
                continue;
            }
            if let Some(version) = decode_record_version(&payload)? {
                self.insert_record_version(version)?;
                continue;
            }
            if let ArchiveRecord::Content(id, bytes) = decode_record(&payload)? {
                if hash_content(&bytes) != id {
                    return Err(ArchiveError::CorruptContent);
                }
                self.contents.insert(id, chunk);
            }
        }
        if !format_seen {
            return Err(ArchiveError::MissingArchiveFormat);
        }
        let scan_elapsed = scan_start.map(|start| start.elapsed());
        let replay_start = profile.then(Instant::now);
        self.rebuild_current_state()?;
        let replay_elapsed = replay_start.map(|start| start.elapsed());
        if let (Some(scan), Some(replay)) = (scan_elapsed, replay_elapsed) {
            report_rebuild(self, scan, replay);
        }
        Ok(())
    }

    pub(crate) fn validate_references(&self) -> Result<(), ArchiveError> {
        for node in self.nodes.iter() {
            if !self.contents.contains(node.content_id) {
                return Err(ArchiveError::MissingContent);
            }
            self.validate_parent(&node.conversation_id, node.parent_id.as_deref())?;
        }
        for branch in self.branches.iter() {
            self.require_node(
                &branch.conversation_id,
                &branch.leaf_node_id,
                ArchiveError::MissingLeaf,
            )?;
            self.branch_nodes(&branch.conversation_id, &branch.leaf_node_id)?;
        }
        self.validate_fragments()?;
        Ok(())
    }

    pub(crate) fn validate_parent(
        &self,
        conversation_id: &str,
        parent_id: Option<&str>,
    ) -> Result<(), ArchiveError> {
        if let Some(parent) = parent_id {
            self.require_node(conversation_id, parent, ArchiveError::MissingParent)?;
        }
        Ok(())
    }

    pub(crate) fn require_node<'a>(
        &'a self,
        conversation_id: &str,
        id: &str,
        error: ArchiveError,
    ) -> Result<&'a Node, ArchiveError> {
        self.nodes.get(conversation_id, id).ok_or(error)
    }

    pub(crate) fn insert_rebuilt_node(&mut self, node: Node) -> Result<(), ArchiveError> {
        self.nodes.insert(node).map(|_| ())
    }

    pub(crate) fn insert_rebuilt_branch_revision(&mut self, branch: Branch) {
        self.branches.put(branch);
    }
}

pub(crate) fn hash_content(bytes: &[u8]) -> ContentId {
    ContentId(Sha256::digest(bytes).into())
}

pub(crate) fn validate_text(value: &str, field: &'static str) -> Result<(), ArchiveError> {
    if value.is_empty() {
        Err(ArchiveError::InvalidField(field))
    } else {
        Ok(())
    }
}
