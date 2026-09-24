use super::{ReliquaryRuntimeHost, ReliquaryRuntimeHostError, operation};
use crate::{GraphRelation, GraphRelationKind, GraphRelationOrigin, Memory, MemoryDraft, MemoryId};
use uuid::Uuid;

impl ReliquaryRuntimeHost {
    pub fn replace_reliquary_knowledge_memory(
        &self,
        memory_id: MemoryId,
        title: String,
        content: String,
        now_ns: i64,
    ) -> Result<Memory, ReliquaryRuntimeHostError> {
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
        let current = runtime.cva.memory(memory_id).map_err(operation)?;
        let relations = runtime.cva.graph_relations();
        let draft = replacement_draft(&current, title, content, now_ns);
        let (replacement, _) = runtime
            .cva
            .publish_memory(None, 0, draft)
            .map_err(operation)?;
        inherit_cva_relations(&mut runtime.cva, &relations, current.id, replacement.id)?;
        archive_cva_memory(&mut runtime.cva, &current, replacement.id, now_ns)?;
        runtime.cva.sync().map_err(operation)?;
        drop(runtime);
        self.wake()?;
        Ok(replacement)
    }

    pub fn replace_phylactery_knowledge_memory(
        &self,
        memory_id: MemoryId,
        title: String,
        content: String,
        now_ns: i64,
    ) -> Result<Memory, ReliquaryRuntimeHostError> {
        let mut slot = self
            .active_execution()?
            .phylactery
            .lock()
            .map_err(|_| ReliquaryRuntimeHostError::LockPoisoned)?;
        let phylactery = slot.as_mut().ok_or_else(|| {
            ReliquaryRuntimeHostError::Operation("Phylactery is unavailable".into())
        })?;
        let current = phylactery.memory(memory_id).map_err(operation)?;
        let relations = phylactery.graph_relations();
        let draft = replacement_draft(&current, title, content, now_ns);
        let replacement = match current.source_ref.clone() {
            Some(source_ref) => {
                phylactery
                    .publish_memory_with_source_ref(None, 0, draft, source_ref)
                    .map_err(operation)?
                    .0
            }
            None => {
                phylactery
                    .publish_memory(None, 0, draft)
                    .map_err(operation)?
                    .0
            }
        };
        inherit_phylactery_relations(phylactery, &relations, current.id, replacement.id)?;
        archive_phylactery_memory(phylactery, &current, replacement.id, now_ns)?;
        phylactery.sync().map_err(operation)?;
        drop(slot);
        self.wake()?;
        Ok(replacement)
    }
}

fn replacement_draft(current: &Memory, title: String, content: String, now_ns: i64) -> MemoryDraft {
    let provenance = if current.mutation_id.starts_with("user_creation:") {
        "user_creation"
    } else {
        "knowledge-replacement"
    };
    MemoryDraft {
        category: current.category.clone(),
        memory_type: current.memory_type.clone(),
        authority_kind: current.authority_kind.clone(),
        title,
        content,
        scope: current.scope.clone(),
        lifecycle_state: current.lifecycle_state.clone(),
        archived: current.archived,
        superseded_by: None,
        parent_id: current.parent_id,
        source_node_id: current.source_node_id.clone(),
        content_source_conversation_id: current.content_source_conversation_id.clone(),
        content_source_node_id: current.content_source_node_id.clone(),
        grounding_source_conversation_id: current.grounding_source_conversation_id.clone(),
        grounding_source_node_id: current.grounding_source_node_id.clone(),
        source_episode_id: current.source_episode_id,
        source_time_ns: current.source_time_ns,
        temporal_status: current.temporal_status.clone(),
        mutation_id: format!("{provenance}:{}", Uuid::new_v4()),
        created_at_ns: current.created_at_ns,
        updated_at_ns: now_ns.max(current.updated_at_ns.saturating_add(1)),
    }
}

fn inherit_cva_relations(
    cva: &mut crate::Cva,
    relations: &[GraphRelation],
    old: MemoryId,
    new: MemoryId,
) -> Result<(), ReliquaryRuntimeHostError> {
    for relation in relations
        .iter()
        .filter(|relation| relation.source == old || relation.target == old)
    {
        let source = if relation.source == old {
            new
        } else {
            relation.source
        };
        let target = if relation.target == old {
            new
        } else {
            relation.target
        };
        let version = cva.graph_version();
        cva.set_memory_relation_with_origin(
            source,
            target,
            relation.kind,
            true,
            relation.origin,
            version,
        )
        .map_err(operation)?;
    }
    let version = cva.graph_version();
    cva.set_memory_relation_with_origin(
        new,
        old,
        GraphRelationKind::Supersedes,
        true,
        GraphRelationOrigin::User,
        version,
    )
    .map_err(operation)?;
    Ok(())
}

fn inherit_phylactery_relations(
    phylactery: &mut crate::Phylactery,
    relations: &[GraphRelation],
    old: MemoryId,
    new: MemoryId,
) -> Result<(), ReliquaryRuntimeHostError> {
    for relation in relations
        .iter()
        .filter(|relation| relation.source == old || relation.target == old)
    {
        let source = if relation.source == old {
            new
        } else {
            relation.source
        };
        let target = if relation.target == old {
            new
        } else {
            relation.target
        };
        let version = phylactery.graph_version();
        phylactery
            .set_memory_relation_with_origin(
                source,
                target,
                relation.kind,
                true,
                relation.origin,
                version,
            )
            .map_err(operation)?;
    }
    let version = phylactery.graph_version();
    phylactery
        .set_memory_relation_with_origin(
            new,
            old,
            GraphRelationKind::Supersedes,
            true,
            GraphRelationOrigin::User,
            version,
        )
        .map_err(operation)?;
    Ok(())
}

fn archive_cva_memory(
    cva: &mut crate::Cva,
    current: &Memory,
    replacement: MemoryId,
    now_ns: i64,
) -> Result<(), ReliquaryRuntimeHostError> {
    cva.publish_memory(
        Some(current.id),
        current.revision,
        retired_draft(current, replacement, now_ns),
    )
    .map_err(operation)?;
    Ok(())
}

fn archive_phylactery_memory(
    phylactery: &mut crate::Phylactery,
    current: &Memory,
    replacement: MemoryId,
    now_ns: i64,
) -> Result<(), ReliquaryRuntimeHostError> {
    phylactery
        .publish_memory(
            Some(current.id),
            current.revision,
            retired_draft(current, replacement, now_ns),
        )
        .map_err(operation)?;
    Ok(())
}

fn retired_draft(current: &Memory, replacement: MemoryId, now_ns: i64) -> MemoryDraft {
    let mut draft = replacement_draft(
        current,
        current.title.clone(),
        current.content.clone(),
        now_ns,
    );
    draft.mutation_id = format!(
        "knowledge-retire:{}:{}",
        hex(&current.id.0),
        hex(&replacement.0)
    );
    draft.lifecycle_state = "archived".into();
    draft.archived = true;
    draft.superseded_by = Some(replacement);
    draft
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}
