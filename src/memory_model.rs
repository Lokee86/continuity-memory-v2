use crate::EpisodeId;
use sha2::{Digest, Sha256};

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct MemoryId(pub [u8; 32]);

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct MemoryBodyId(pub [u8; 32]);

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct MemoryRevisionId {
    pub memory_id: MemoryId,
    pub revision: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MemoryDraft {
    pub category: String,
    pub memory_type: String,
    pub authority_kind: String,
    pub title: String,
    pub content: String,
    pub scope: String,
    pub lifecycle_state: String,
    pub archived: bool,
    pub superseded_by: Option<MemoryId>,
    pub parent_id: Option<MemoryId>,
    pub source_node_id: Option<String>,
    pub content_source_conversation_id: Option<String>,
    pub content_source_node_id: Option<String>,
    pub grounding_source_conversation_id: Option<String>,
    pub grounding_source_node_id: Option<String>,
    pub source_episode_id: Option<EpisodeId>,
    pub mutation_id: String,
    pub created_at_ns: i64,
    pub updated_at_ns: i64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Memory {
    pub id: MemoryId,
    pub revision: u64,
    pub category: String,
    pub memory_type: String,
    pub authority_kind: String,
    pub title: String,
    pub content: String,
    pub scope: String,
    pub lifecycle_state: String,
    pub archived: bool,
    pub superseded_by: Option<MemoryId>,
    pub parent_id: Option<MemoryId>,
    pub source_node_id: Option<String>,
    pub content_source_conversation_id: Option<String>,
    pub content_source_node_id: Option<String>,
    pub grounding_source_conversation_id: Option<String>,
    pub grounding_source_node_id: Option<String>,
    pub source_episode_id: Option<EpisodeId>,
    pub mutation_id: String,
    pub created_at_ns: i64,
    pub updated_at_ns: i64,
    pub global_version: u64,
    pub memory_version: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MemoryStats {
    pub memories: usize,
    pub revisions: usize,
    pub bodies: usize,
    pub memory_version: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct MemoryRecord {
    pub id: MemoryId,
    pub revision: u64,
    pub body_id: MemoryBodyId,
    pub category: String,
    pub memory_type: String,
    pub authority_kind: String,
    pub scope: String,
    pub lifecycle_state: String,
    pub archived: bool,
    pub superseded_by: Option<MemoryId>,
    pub parent_id: Option<MemoryId>,
    pub source_node_id: Option<String>,
    pub content_source_conversation_id: Option<String>,
    pub content_source_node_id: Option<String>,
    pub grounding_source_conversation_id: Option<String>,
    pub grounding_source_node_id: Option<String>,
    pub source_episode_id: Option<EpisodeId>,
    pub mutation_id: String,
    pub created_at_ns: i64,
    pub updated_at_ns: i64,
    pub global_version: u64,
    pub memory_version: u64,
}

pub(crate) fn memory_id(mutation_id: &str) -> MemoryId {
    let mut hash = Sha256::new();
    hash.update(b"continuity-memory-id\0");
    hash.update((mutation_id.len() as u64).to_le_bytes());
    hash.update(mutation_id.as_bytes());
    MemoryId(hash.finalize().into())
}

pub(crate) fn memory_body_bytes(title: &str, content: &str) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(16 + title.len() + content.len());
    bytes.extend_from_slice(&(title.len() as u64).to_le_bytes());
    bytes.extend_from_slice(title.as_bytes());
    bytes.extend_from_slice(&(content.len() as u64).to_le_bytes());
    bytes.extend_from_slice(content.as_bytes());
    bytes
}

pub(crate) fn memory_body_id(title: &str, content: &str) -> MemoryBodyId {
    MemoryBodyId(Sha256::digest(memory_body_bytes(title, content)).into())
}
