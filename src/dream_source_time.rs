use crate::{Cva, Memory};

pub(crate) fn source_timestamp_ns(cva: &Cva, memory: &Memory) -> Option<i64> {
    if let Some(source_time_ns) = memory.source_time_ns {
        return Some(source_time_ns);
    }
    if let (Some(conversation), Some(node_id)) = (
        memory.content_source_conversation_id.as_deref(),
        memory.content_source_node_id.as_deref(),
    ) && let Some(node) = cva.archive.nodes.get(conversation, node_id)
    {
        return Some(node.timestamp_ns);
    }
    if let (Some(episode_id), Some(node_id)) =
        (memory.source_episode_id, memory.source_node_id.as_deref())
        && let Some(episode) = cva.archive.episodes.get(episode_id)
        && let Some(node) = cva.archive.nodes.get(&episode.conversation_id, node_id)
    {
        return Some(node.timestamp_ns);
    }
    memory
        .source_episode_id
        .and_then(|id| cva.archive.episodes.get(id))
        .map(|episode| episode.source_through_ns)
}
