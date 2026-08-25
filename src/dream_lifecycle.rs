use crate::dream_canonical::{
    canonical_superseders, corroborated_representative, duplicate_component_has_active_peer,
    explicit_authority_promotes,
};
use crate::{
    Cva, DreamLifecycleError, DreamLifecycleResult, GraphRelation, GraphRelationKind, Memory,
    MemoryDraft, MemoryId,
};

impl Cva {
    pub fn reconcile_dream_lifecycle(
        &mut self,
        source_id: MemoryId,
    ) -> Result<DreamLifecycleResult, DreamLifecycleError> {
        let relations = self.graph_relations();
        let mut revised = Vec::new();
        let mut archived = Vec::new();
        let mut canonicalized = canonical_superseders(self, source_id, &relations)?;

        let mut superseded_targets: Vec<_> = relations
            .iter()
            .filter(|relation| {
                relation.kind == GraphRelationKind::Supersedes
                    && (relation.source == source_id || relation.target == source_id)
            })
            .map(|relation| relation.target)
            .collect();
        superseded_targets.sort_by_key(|id| id.0);
        superseded_targets.dedup();

        for target in superseded_targets {
            let superseded_by = unique_superseder(&relations, target);
            if let Some(memory) = self.revise_lifecycle(target, "archived", true, superseded_by)? {
                archived.push(target);
                revised.push(memory);
            }
        }

        let source = self.memory(source_id)?;
        if !source.archived
            && source.lifecycle_state == "extracted"
            && duplicate_component_has_active_peer(self, source_id, &relations)?
            && let Some(memory) =
                self.revise_lifecycle(source_id, "archived", true, source.superseded_by)?
        {
            archived.push(source_id);
            revised.push(memory);
        }

        let source = self.memory(source_id)?;
        if !source.archived && explicit_authority_promotes(&source) {
            canonicalized.push(source_id);
        }
        if let Some(representative) = corroborated_representative(self, source_id, &relations)? {
            canonicalized.push(representative);
        }
        canonicalized.sort_by_key(|id| id.0);
        canonicalized.dedup();

        let mut promoted = Vec::new();
        for id in canonicalized {
            let current = self.memory(id)?;
            if current.archived || current.lifecycle_state == "canonical" {
                continue;
            }
            if let Some(memory) =
                self.revise_lifecycle(id, "canonical", false, current.superseded_by)?
            {
                revised.push(memory);
                promoted.push(id);
            }
        }

        let source = self.memory(source_id)?;
        let promoted_to_knowledge = if !source.archived && source.lifecycle_state == "extracted" {
            if let Some(memory) =
                self.revise_lifecycle(source_id, "knowledge", false, source.superseded_by)?
            {
                revised.push(memory);
                true
            } else {
                false
            }
        } else {
            false
        };

        archived.sort_by_key(|id| id.0);
        archived.dedup();
        promoted.sort_by_key(|id| id.0);
        Ok(DreamLifecycleResult {
            source: self.memory(source_id)?,
            revised,
            archived,
            canonicalized: promoted,
            promoted_to_knowledge,
        })
    }

    fn revise_lifecycle(
        &mut self,
        id: MemoryId,
        lifecycle_state: &str,
        archived: bool,
        superseded_by: Option<MemoryId>,
    ) -> Result<Option<Memory>, DreamLifecycleError> {
        let current = self.memory(id)?;
        if current.lifecycle_state == lifecycle_state
            && current.archived == archived
            && current.superseded_by == superseded_by
        {
            return Ok(None);
        }
        let graph_version = self.graph_version();
        let draft = lifecycle_draft(
            &current,
            lifecycle_state,
            archived,
            superseded_by,
            graph_version,
        );
        let (memory, changed) = self.publish_memory(Some(id), current.revision, draft)?;
        Ok(changed.then_some(memory))
    }
}

fn unique_superseder(relations: &[GraphRelation], target: MemoryId) -> Option<MemoryId> {
    let mut sources: Vec<_> = relations
        .iter()
        .filter(|relation| {
            relation.kind == GraphRelationKind::Supersedes && relation.target == target
        })
        .map(|relation| relation.source)
        .collect();
    sources.sort_by_key(|id| id.0);
    sources.dedup();
    match sources.as_slice() {
        [source] => Some(*source),
        _ => None,
    }
}

fn lifecycle_draft(
    memory: &Memory,
    lifecycle_state: &str,
    archived: bool,
    superseded_by: Option<MemoryId>,
    graph_version: u64,
) -> MemoryDraft {
    MemoryDraft {
        category: memory.category.clone(),
        memory_type: memory.memory_type.clone(),
        authority_kind: memory.authority_kind.clone(),
        title: memory.title.clone(),
        content: memory.content.clone(),
        scope: memory.scope.clone(),
        lifecycle_state: lifecycle_state.into(),
        archived,
        superseded_by,
        parent_id: memory.parent_id,
        source_node_id: memory.source_node_id.clone(),
        content_source_conversation_id: memory.content_source_conversation_id.clone(),
        content_source_node_id: memory.content_source_node_id.clone(),
        grounding_source_conversation_id: memory.grounding_source_conversation_id.clone(),
        grounding_source_node_id: memory.grounding_source_node_id.clone(),
        source_episode_id: memory.source_episode_id,
        mutation_id: lifecycle_mutation_id(
            memory.id,
            lifecycle_state,
            superseded_by,
            graph_version,
        ),
        created_at_ns: memory.created_at_ns,
        updated_at_ns: memory.updated_at_ns.saturating_add(1),
    }
}

fn lifecycle_mutation_id(
    id: MemoryId,
    lifecycle_state: &str,
    superseded_by: Option<MemoryId>,
    graph_version: u64,
) -> String {
    let id = bytes_hex(&id.0);
    let superseder = superseded_by
        .map(|value| bytes_hex(&value.0))
        .unwrap_or_else(|| "none".into());
    format!("dream-lifecycle:{id}:{graph_version}:{lifecycle_state}:{superseder}")
}

fn bytes_hex(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        use std::fmt::Write;
        let _ = write!(&mut out, "{byte:02x}");
    }
    out
}
