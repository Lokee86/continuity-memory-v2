use super::{ReliquaryRuntimeHost, ReliquaryRuntimeHostError, operation};
use crate::{Cva, Memory, MemoryId, MemoryProvenance, ResolvedTurn};

impl ReliquaryRuntimeHost {
    pub fn read_reliquary_memory_provenance(
        &self,
        memory_id: MemoryId,
    ) -> Result<MemoryProvenance, ReliquaryRuntimeHostError> {
        let runtime = self
            .active_execution()?
            .runtime
            .as_ref()
            .cloned()
            .ok_or_else(|| {
                ReliquaryRuntimeHostError::Operation("Reliquary runtime is unavailable".into())
            })?;
        let mut runtime = runtime
            .lock()
            .map_err(|_| ReliquaryRuntimeHostError::LockPoisoned)?;
        let memory = runtime.cva.memory(memory_id).map_err(operation)?;
        resolve_memory_provenance(&mut runtime.cva, memory)
    }

    pub fn resolve_external_memory_provenance(
        &self,
        mut memory: Memory,
    ) -> Result<MemoryProvenance, ReliquaryRuntimeHostError> {
        let source_ref = memory.source_ref.clone().ok_or_else(|| {
            ReliquaryRuntimeHostError::Operation(
                "Memory does not have an external source reference".into(),
            )
        })?;
        let runtime = self
            .active_execution()?
            .runtime
            .as_ref()
            .cloned()
            .ok_or_else(|| {
                ReliquaryRuntimeHostError::Operation("Reliquary runtime is unavailable".into())
            })?;
        let mut runtime = runtime
            .lock()
            .map_err(|_| ReliquaryRuntimeHostError::LockPoisoned)?;
        if runtime.cva.owner_id().as_deref() != Some(source_ref.owner_id.as_str()) {
            return Err(ReliquaryRuntimeHostError::Operation(
                "external Memory provenance belongs to a different Reliquary".into(),
            ));
        }
        memory.source_episode_id = Some(source_ref.source_episode_id);
        memory.source_node_id = Some(source_ref.source_node_id);
        memory.content_source_conversation_id = source_ref.content_source_conversation_id;
        memory.content_source_node_id = source_ref.content_source_node_id;
        memory.grounding_source_conversation_id = source_ref.grounding_source_conversation_id;
        memory.grounding_source_node_id = source_ref.grounding_source_node_id;
        resolve_memory_provenance(&mut runtime.cva, memory)
    }
}

fn resolve_memory_provenance(
    cva: &mut Cva,
    memory: Memory,
) -> Result<MemoryProvenance, ReliquaryRuntimeHostError> {
    let source_episode = match memory.source_episode_id {
        Some(id) => Some(cva.episode(id).cloned().ok_or_else(|| {
            ReliquaryRuntimeHostError::Operation("Memory source Episode is unavailable".into())
        })?),
        None => None,
    };
    let source_episode_turns = match source_episode.as_ref() {
        Some(episode) => cva.episode_turns(episode.id).map_err(operation)?,
        None => Vec::new(),
    };
    let source_turn = memory
        .source_node_id
        .as_deref()
        .map(|node_id| {
            source_episode_turns
                .iter()
                .find(|turn| turn.node_id == node_id)
                .cloned()
                .ok_or_else(|| {
                    ReliquaryRuntimeHostError::Operation(
                        "Memory source turn is unavailable from its source Episode".into(),
                    )
                })
        })
        .transpose()?;
    let content_source_turn = resolve_source_turn(
        cva,
        memory.content_source_conversation_id.as_deref(),
        memory.content_source_node_id.as_deref(),
        "content source",
    )?;
    let grounding_source_turn = resolve_source_turn(
        cva,
        memory.grounding_source_conversation_id.as_deref(),
        memory.grounding_source_node_id.as_deref(),
        "grounding source",
    )?;
    Ok(MemoryProvenance {
        memory,
        source_episode,
        source_turn,
        source_episode_turns,
        content_source_turn,
        grounding_source_turn,
    })
}

fn resolve_source_turn(
    cva: &mut Cva,
    conversation_id: Option<&str>,
    node_id: Option<&str>,
    label: &str,
) -> Result<Option<ResolvedTurn>, ReliquaryRuntimeHostError> {
    let (conversation_id, node_id) = match (conversation_id, node_id) {
        (Some(conversation_id), Some(node_id)) => (conversation_id, node_id),
        (None, None) => return Ok(None),
        _ => {
            return Err(ReliquaryRuntimeHostError::Operation(format!(
                "Memory {label} provenance is incomplete"
            )));
        }
    };
    let turn = cva
        .conversation_turns(conversation_id, node_id)
        .map_err(operation)?
        .into_iter()
        .last()
        .filter(|turn| turn.node_id == node_id)
        .ok_or_else(|| {
            ReliquaryRuntimeHostError::Operation(format!("Memory {label} turn is unavailable"))
        })?;
    Ok(Some(turn))
}
