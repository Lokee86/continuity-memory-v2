use crate::memory_store::MemoryStore;
use crate::{Archive, Container, Memory, MemoryDraft, MemoryError, MemoryId};

pub(crate) fn publish_memory_parts(
    archive: &Archive,
    memories: &mut MemoryStore,
    container: &mut Container,
    id: Option<MemoryId>,
    expected_revision: u64,
    draft: MemoryDraft,
) -> Result<(Memory, bool), MemoryError> {
    validate_memory_provenance(archive, &draft)?;
    memories.publish(container, id, expected_revision, draft)
}

fn validate_memory_provenance(archive: &Archive, draft: &MemoryDraft) -> Result<(), MemoryError> {
    match (draft.source_episode_id, draft.source_node_id.as_deref()) {
        (Some(episode_id), Some(node_id)) => {
            if !archive
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
        (Some(conversation_id), Some(node_id)) if archive.has_node(conversation_id, node_id) => {}
        (None, None) => {}
        _ => return Err(MemoryError::InvalidProvenance),
    }
    match (
        draft.grounding_source_conversation_id.as_deref(),
        draft.grounding_source_node_id.as_deref(),
    ) {
        (Some(conversation_id), Some(node_id)) if archive.has_node(conversation_id, node_id) => {}
        (None, None) => {}
        _ => return Err(MemoryError::InvalidProvenance),
    }
    Ok(())
}
