use crate::community_store::CommunityStore;
use crate::compatibility_profile_store::CompatibilityProfileStore;
use crate::dream_cooldown::DreamCooldownStore;
use crate::dream_duplicate_index::DuplicateIndex;
use crate::graph_store::GraphStore;
use crate::memory_store::MemoryStore;
use crate::memory_vector_store::MemoryVectorStore;
use crate::packed_vector_store::PackedVectorStore;
use crate::{
    Container, Memory, MemoryBodyId, MemoryDraft, MemoryError, MemoryId, MemoryStats,
    PhylacteryError,
};

pub struct Phylactery {
    pub(crate) container: Container,
    pub(crate) memories: MemoryStore,
    pub(crate) graph: GraphStore,
    pub(crate) communities: CommunityStore,
    pub(crate) duplicate_index: DuplicateIndex,
    pub(crate) dream_cooldowns: DreamCooldownStore,
    pub(crate) packed_vectors: PackedVectorStore,
    pub(crate) memory_vectors: MemoryVectorStore,
    pub(crate) compatibility_profiles: CompatibilityProfileStore,
}

impl Phylactery {
    pub fn owner_id(&self) -> Option<String> {
        self.container.owner_id()
    }

    pub fn owner_uuid(&self) -> Option<[u8; 16]> {
        self.container.owner_uuid()
    }

    pub fn publish_memory(
        &mut self,
        id: Option<MemoryId>,
        expected_revision: u64,
        draft: MemoryDraft,
    ) -> Result<(Memory, bool), MemoryError> {
        validate_phylactery_provenance(&draft)?;
        self.memories
            .publish(&mut self.container, id, expected_revision, draft)
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

    pub fn memory_body_id(&self, id: MemoryId) -> Result<MemoryBodyId, MemoryError> {
        self.memories.current_body_id(id)
    }

    pub fn sync(&self) -> Result<(), PhylacteryError> {
        self.container.sync()?;
        Ok(())
    }
}

fn validate_phylactery_provenance(draft: &MemoryDraft) -> Result<(), MemoryError> {
    if draft.source_episode_id.is_some()
        || draft.source_node_id.is_some()
        || draft.content_source_conversation_id.is_some()
        || draft.content_source_node_id.is_some()
        || draft.grounding_source_conversation_id.is_some()
        || draft.grounding_source_node_id.is_some()
    {
        return Err(MemoryError::InvalidProvenance);
    }
    Ok(())
}
