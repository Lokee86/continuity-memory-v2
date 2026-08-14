use crate::archive_lookup::DenseLookup;
use crate::{ArchiveError, Branch, Node};
use std::hash::BuildHasher;

#[derive(Default)]
pub(crate) struct NodeIndex {
    records: Vec<Node>,
    lookup: DenseLookup,
}

impl NodeIndex {
    pub(crate) fn len(&self) -> usize {
        self.records.len()
    }

    pub(crate) fn get(&self, conversation_id: &str, id: &str) -> Option<&Node> {
        let hash = self.lookup.hash(&(conversation_id, id));
        self.lookup
            .find(hash, |index| {
                let node = &self.records[index];
                node.conversation_id == conversation_id && node.id == id
            })
            .map(|index| &self.records[index])
    }

    pub(crate) fn insert(&mut self, node: Node) -> Result<bool, ArchiveError> {
        if let Some(existing) = self.get(&node.conversation_id, &node.id) {
            return if existing == &node {
                Ok(false)
            } else {
                Err(ArchiveError::ConflictingNode)
            };
        }
        let hash = self
            .lookup
            .hash(&(node.conversation_id.as_str(), node.id.as_str()));
        let index = self.records.len();
        self.records.push(node);
        let records = &self.records;
        self.lookup.insert(hash, index, |state, found| {
            let node = &records[found];
            state.hash_one(&(node.conversation_id.as_str(), node.id.as_str()))
        });
        Ok(true)
    }

    pub(crate) fn iter(&self) -> impl Iterator<Item = &Node> {
        self.records.iter()
    }
}

#[derive(Default)]
pub(crate) struct BranchIndex {
    records: Vec<Branch>,
    lookup: DenseLookup,
}

impl BranchIndex {
    pub(crate) fn len(&self) -> usize {
        self.records.len()
    }

    pub(crate) fn get(&self, conversation_id: &str, id: &str) -> Option<&Branch> {
        let hash = self.lookup.hash(&(conversation_id, id));
        self.lookup
            .find(hash, |index| {
                let branch = &self.records[index];
                branch.conversation_id == conversation_id && branch.id == id
            })
            .map(|index| &self.records[index])
    }

    pub(crate) fn put(&mut self, branch: Branch) {
        let hash = self
            .lookup
            .hash(&(branch.conversation_id.as_str(), branch.id.as_str()));
        if let Some(index) = self.lookup.find(hash, |index| {
            let existing = &self.records[index];
            existing.conversation_id == branch.conversation_id && existing.id == branch.id
        }) {
            self.records[index] = branch;
            return;
        }
        let index = self.records.len();
        self.records.push(branch);
        let records = &self.records;
        self.lookup.insert(hash, index, |state, found| {
            let branch = &records[found];
            state.hash_one(&(branch.conversation_id.as_str(), branch.id.as_str()))
        });
    }

    pub(crate) fn iter(&self) -> impl Iterator<Item = &Branch> {
        self.records.iter()
    }
}
