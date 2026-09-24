use super::{ReliquaryRuntimeHost, ReliquaryRuntimeHostError, operation};
use crate::{
    CommunityId, CommunitySemanticName, CommunitySnapshot, Entity, Memory, MemoryDraft,
    SemanticGraphRelation,
};
use uuid::Uuid;

pub type RuntimeKnowledgeState = (
    Vec<Memory>,
    Vec<Entity>,
    Vec<SemanticGraphRelation>,
    Option<CommunitySnapshot>,
    Vec<CommunitySemanticName>,
);

impl ReliquaryRuntimeHost {
    pub fn read_reliquary_knowledge(
        &self,
    ) -> Result<RuntimeKnowledgeState, ReliquaryRuntimeHostError> {
        let owner_id = self.active_rel_id().ok_or_else(|| {
            ReliquaryRuntimeHostError::Operation("Reliquary runtime has no active REL".into())
        })?;
        self.read_reliquary_knowledge_for(&owner_id)
    }

    pub fn read_reliquary_knowledge_for(
        &self,
        owner_id: &str,
    ) -> Result<RuntimeKnowledgeState, ReliquaryRuntimeHostError> {
        let runtime = self.runtime_for_owner(owner_id)?;
        let mut runtime = runtime
            .lock()
            .map_err(|_| ReliquaryRuntimeHostError::LockPoisoned)?;
        let memories = runtime
            .cva
            .memory_ids()
            .into_iter()
            .map(|id| runtime.cva.memory(id).map_err(operation))
            .collect::<Result<Vec<_>, _>>()?;
        Ok((
            memories,
            runtime
                .cva
                .entities()
                .into_iter()
                .filter(|entity| entity.kind != crate::entity_principal::PRINCIPAL_ENTITY_KIND)
                .collect(),
            runtime.cva.semantic_graph_relations(),
            runtime.cva.community_snapshot(),
            runtime.cva.community_semantic_names(),
        ))
    }

    pub fn read_phylactery_knowledge(
        &self,
    ) -> Result<Option<RuntimeKnowledgeState>, ReliquaryRuntimeHostError> {
        let mut slot = self
            .phylactery
            .lock()
            .map_err(|_| ReliquaryRuntimeHostError::LockPoisoned)?;
        let Some(phylactery) = slot.as_mut() else {
            return Ok(None);
        };
        let memories = phylactery
            .memory_ids()
            .into_iter()
            .map(|id| phylactery.memory(id).map_err(operation))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Some((
            memories,
            phylactery
                .entities()
                .into_iter()
                .filter(|entity| entity.kind != crate::entity_principal::PRINCIPAL_ENTITY_KIND)
                .collect(),
            phylactery.semantic_graph_relations(),
            phylactery.community_snapshot(),
            phylactery.community_semantic_names(),
        )))
    }

    pub fn create_reliquary_knowledge_memory(
        &self,
        title: String,
        content: String,
        category: String,
        memory_type: String,
        now_ns: i64,
    ) -> Result<Memory, ReliquaryRuntimeHostError> {
        let owner_id = self.active_rel_id().ok_or_else(|| {
            ReliquaryRuntimeHostError::Operation("Reliquary runtime has no active REL".into())
        })?;
        self.create_reliquary_knowledge_memory_for(
            &owner_id,
            title,
            content,
            category,
            memory_type,
            now_ns,
        )
    }

    pub fn create_reliquary_knowledge_memory_for(
        &self,
        owner_id: &str,
        title: String,
        content: String,
        category: String,
        memory_type: String,
        now_ns: i64,
    ) -> Result<Memory, ReliquaryRuntimeHostError> {
        let runtime = self.runtime_for_owner(owner_id)?;
        let mut runtime = runtime
            .lock()
            .map_err(|_| ReliquaryRuntimeHostError::LockPoisoned)?;
        let draft = user_creation_draft(title, content, category, memory_type, now_ns);
        let (memory, _) = runtime
            .cva
            .publish_memory(None, 0, draft)
            .map_err(operation)?;
        runtime.cva.sync().map_err(operation)?;
        drop(runtime);
        self.wake_owner(owner_id)?;
        Ok(memory)
    }

    pub fn create_phylactery_knowledge_memory(
        &self,
        title: String,
        content: String,
        category: String,
        memory_type: String,
        now_ns: i64,
    ) -> Result<Memory, ReliquaryRuntimeHostError> {
        let mut slot = self
            .phylactery
            .lock()
            .map_err(|_| ReliquaryRuntimeHostError::LockPoisoned)?;
        let phylactery = slot.as_mut().ok_or_else(|| {
            ReliquaryRuntimeHostError::Operation("Phylactery is unavailable".into())
        })?;
        let draft = user_creation_draft(title, content, category, memory_type, now_ns);
        let (memory, _) = phylactery
            .publish_memory(None, 0, draft)
            .map_err(operation)?;
        phylactery.sync().map_err(operation)?;
        drop(slot);
        if self.active_rel_id().is_some() {
            self.wake()?;
        }
        Ok(memory)
    }

    pub fn rename_reliquary_community(
        &self,
        community_id: CommunityId,
        name: String,
    ) -> Result<(), ReliquaryRuntimeHostError> {
        let owner_id = self.active_rel_id().ok_or_else(|| {
            ReliquaryRuntimeHostError::Operation("Reliquary runtime has no active REL".into())
        })?;
        self.rename_reliquary_community_for(&owner_id, community_id, name)
    }

    pub fn rename_reliquary_community_for(
        &self,
        owner_id: &str,
        community_id: CommunityId,
        name: String,
    ) -> Result<(), ReliquaryRuntimeHostError> {
        let runtime = self.runtime_for_owner(owner_id)?;
        let mut runtime = runtime
            .lock()
            .map_err(|_| ReliquaryRuntimeHostError::LockPoisoned)?;
        runtime
            .cva
            .set_community_name(community_id, name)
            .map_err(operation)?;
        runtime.cva.sync().map_err(operation)?;
        Ok(())
    }

    pub fn rename_phylactery_community(
        &self,
        community_id: CommunityId,
        name: String,
    ) -> Result<(), ReliquaryRuntimeHostError> {
        let mut slot = self
            .phylactery
            .lock()
            .map_err(|_| ReliquaryRuntimeHostError::LockPoisoned)?;
        let phylactery = slot.as_mut().ok_or_else(|| {
            ReliquaryRuntimeHostError::Operation("Phylactery is unavailable".into())
        })?;
        phylactery
            .set_community_name(community_id, name)
            .map_err(operation)?;
        phylactery.sync().map_err(operation)?;
        Ok(())
    }
}

fn user_creation_draft(
    title: String,
    content: String,
    category: String,
    memory_type: String,
    now_ns: i64,
) -> MemoryDraft {
    MemoryDraft {
        category,
        memory_type,
        authority_kind: "direct".into(),
        title,
        content,
        scope: "private".into(),
        lifecycle_state: "extracted".into(),
        archived: false,
        superseded_by: None,
        parent_id: None,
        source_node_id: None,
        content_source_conversation_id: None,
        content_source_node_id: None,
        grounding_source_conversation_id: None,
        grounding_source_node_id: None,
        source_episode_id: None,
        source_time_ns: Some(now_ns),
        temporal_status: "unknown".into(),
        mutation_id: format!("user_creation:{}", Uuid::new_v4()),
        created_at_ns: now_ns,
        updated_at_ns: now_ns,
    }
}
