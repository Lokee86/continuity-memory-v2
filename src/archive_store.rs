use crate::archive_codec::{ArchiveRecord, decode_record, encode_content};
use crate::{Archive, ArchiveError, Container, ContentId, Node};
use sha2::{Digest, Sha256};
use std::collections::HashSet;

impl Archive {
    pub(crate) fn empty() -> Self {
        Self {
            contents: Default::default(),
            nodes: Default::default(),
            branches: Default::default(),
            fragments: Default::default(),
            record_versions: Vec::new(),
            next_archive_version: 1,
        }
    }

    pub(crate) fn put_content(
        &mut self,
        container: &mut Container,
        id: ContentId,
        content: &str,
    ) -> Result<(), ArchiveError> {
        if self.contents.contains(id) {
            if self.content(container, id)? != content {
                return Err(ArchiveError::HashCollision);
            }
            return Ok(());
        }
        let chunk = container.append(&encode_content(id, content.as_bytes())?)?;
        self.contents.insert(id, chunk);
        Ok(())
    }

    pub(crate) fn content(
        &self,
        container: &mut Container,
        id: ContentId,
    ) -> Result<String, ArchiveError> {
        let chunk = self.contents.get(id).ok_or(ArchiveError::MissingContent)?;
        match decode_record(&container.read(chunk)?)? {
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
