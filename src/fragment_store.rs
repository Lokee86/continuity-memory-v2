use crate::archive_codec::encode_fragment;
use crate::{Archive, ArchiveError, Container, Fragment, FragmentId, Node, ResolvedTurn};
use sha2::{Digest, Sha256};
use std::collections::HashSet;

impl Archive {
    pub(crate) fn put_fragment(
        &mut self,
        container: &mut Container,
        fragment: Fragment,
    ) -> Result<bool, ArchiveError> {
        self.validate_fragment(&fragment)?;
        if let Some(existing) = self.fragments.get(fragment.id) {
            return if existing == &fragment {
                Ok(false)
            } else {
                Err(ArchiveError::ConflictingFragment)
            };
        }
        let record = container.append(&encode_fragment(&fragment)?)?;
        self.publish_record(container, record)?;
        self.fragments.insert(fragment)?;
        Ok(true)
    }

    pub(crate) fn fragment_nodes_for(
        &self,
        fragment: &Fragment,
    ) -> Result<Vec<Node>, ArchiveError> {
        let mut nodes = Vec::new();
        let mut seen = HashSet::new();
        let mut current = fragment.end_node_id.clone();
        loop {
            if !seen.insert(current.clone()) {
                return Err(ArchiveError::NodeCycle);
            }
            let node = self
                .nodes
                .get(&fragment.conversation_id, &current)
                .ok_or(ArchiveError::MissingNode)?
                .clone();
            let done = node.id == fragment.start_node_id;
            current = node.parent_id.clone().unwrap_or_default();
            nodes.push(node);
            if done {
                break;
            }
            if current.is_empty() {
                return Err(ArchiveError::InvalidFragmentRange);
            }
        }
        nodes.reverse();
        Ok(nodes)
    }

    pub(crate) fn validate_fragments(&self) -> Result<(), ArchiveError> {
        for fragment in self.fragments.iter() {
            self.validate_fragment(fragment)?;
        }
        Ok(())
    }

    pub(crate) fn validate_fragment(&self, fragment: &Fragment) -> Result<(), ArchiveError> {
        if fragment.id
            != fragment_id(
                &fragment.conversation_id,
                &fragment.start_node_id,
                &fragment.end_node_id,
            )
        {
            return Err(ArchiveError::InvalidFragmentId);
        }
        self.fragment_nodes_for(fragment)?;
        Ok(())
    }

    pub(crate) fn fragment_turns(
        &self,
        container: &mut Container,
        id: FragmentId,
    ) -> Result<Vec<ResolvedTurn>, ArchiveError> {
        let fragment = self
            .fragments
            .get(id)
            .ok_or(ArchiveError::MissingFragment)?
            .clone();
        let nodes = self.fragment_nodes_for(&fragment)?;
        nodes
            .into_iter()
            .map(|node| {
                let content = self.content(container, node.content_id)?;
                Ok(ResolvedTurn {
                    node_id: node.id,
                    role: node.role,
                    timestamp_ns: node.timestamp_ns,
                    content,
                })
            })
            .collect()
    }

    pub(crate) fn fragment_text(
        &self,
        container: &mut Container,
        id: FragmentId,
    ) -> Result<String, ArchiveError> {
        let turns = self.fragment_turns(container, id)?;
        Ok(turns
            .into_iter()
            .map(|turn| format!("{}: {}", turn.role, turn.content))
            .collect::<Vec<_>>()
            .join("\n"))
    }

    pub fn fragments(&self) -> Vec<Fragment> {
        let mut fragments: Vec<_> = self.fragments.iter().cloned().collect();
        fragments.sort_by_key(|fragment| fragment.id.0);
        fragments
    }
}

pub(crate) fn fragment_id(
    conversation_id: &str,
    start_node_id: &str,
    end_node_id: &str,
) -> FragmentId {
    let mut hash = Sha256::new();
    for value in [conversation_id, start_node_id, end_node_id] {
        hash.update(value.as_bytes());
        hash.update([0]);
    }
    FragmentId(hash.finalize().into())
}
