use super::{
    ReliquaryRuntimeHost, ReliquaryRuntimeHostError, knowledge::RuntimeKnowledgeState,
    semantic_access::RuntimeSemanticOwnerKind,
};
use crate::{CommunityId, GraphRelationKind, Memory, MemoryId};

impl ReliquaryRuntimeHost {
    pub fn read_knowledge_for_owner(
        &self,
        owner_id: &str,
    ) -> Result<RuntimeKnowledgeState, ReliquaryRuntimeHostError> {
        match self.semantic_owner(owner_id)?.kind {
            RuntimeSemanticOwnerKind::Reliquary => self.read_reliquary_knowledge_for(owner_id),
            RuntimeSemanticOwnerKind::Phylactery => {
                self.read_phylactery_knowledge()?.ok_or_else(|| {
                    ReliquaryRuntimeHostError::Operation("Phylactery is unavailable".into())
                })
            }
        }
    }

    pub fn create_knowledge_memory_for_owner(
        &self,
        owner_id: &str,
        title: String,
        content: String,
        category: String,
        memory_type: String,
    ) -> Result<Memory, ReliquaryRuntimeHostError> {
        let now_ns = crate::insomnia::runtime_step::now_ns();
        match self.semantic_owner(owner_id)?.kind {
            RuntimeSemanticOwnerKind::Reliquary => self.create_reliquary_knowledge_memory_for(
                owner_id,
                title,
                content,
                category,
                memory_type,
                now_ns,
            ),
            RuntimeSemanticOwnerKind::Phylactery => self.create_phylactery_knowledge_memory(
                title,
                content,
                category,
                memory_type,
                now_ns,
            ),
        }
    }

    pub fn replace_knowledge_memory_for_owner(
        &self,
        owner_id: &str,
        memory_id: MemoryId,
        title: String,
        content: String,
    ) -> Result<Memory, ReliquaryRuntimeHostError> {
        let now_ns = crate::insomnia::runtime_step::now_ns();
        match self.semantic_owner(owner_id)?.kind {
            RuntimeSemanticOwnerKind::Reliquary => self.replace_reliquary_knowledge_memory_for(
                owner_id, memory_id, title, content, now_ns,
            ),
            RuntimeSemanticOwnerKind::Phylactery => {
                self.replace_phylactery_knowledge_memory(memory_id, title, content, now_ns)
            }
        }
    }

    pub fn mutate_knowledge_relation_for_owner(
        &self,
        owner_id: &str,
        source: MemoryId,
        target: MemoryId,
        old_kind: Option<GraphRelationKind>,
        new_kind: Option<GraphRelationKind>,
    ) -> Result<(), ReliquaryRuntimeHostError> {
        match self.semantic_owner(owner_id)?.kind {
            RuntimeSemanticOwnerKind::Reliquary => self.mutate_reliquary_knowledge_relation_for(
                owner_id, source, target, old_kind, new_kind,
            ),
            RuntimeSemanticOwnerKind::Phylactery => {
                self.mutate_phylactery_knowledge_relation(source, target, old_kind, new_kind)
            }
        }
    }

    pub fn rename_knowledge_community_for_owner(
        &self,
        owner_id: &str,
        community_id: CommunityId,
        name: String,
    ) -> Result<(), ReliquaryRuntimeHostError> {
        match self.semantic_owner(owner_id)?.kind {
            RuntimeSemanticOwnerKind::Reliquary => {
                self.rename_reliquary_community_for(owner_id, community_id, name)
            }
            RuntimeSemanticOwnerKind::Phylactery => {
                self.rename_phylactery_community(community_id, name)
            }
        }
    }

    pub fn knowledge_memory_for_owner(
        &self,
        owner_id: &str,
        memory_id: MemoryId,
    ) -> Result<Memory, ReliquaryRuntimeHostError> {
        match self.semantic_owner(owner_id)?.kind {
            RuntimeSemanticOwnerKind::Reliquary => self.with_runtime_for(owner_id, |runtime| {
                runtime.cva.memory(memory_id).map_err(super::operation)
            }),
            RuntimeSemanticOwnerKind::Phylactery => {
                let state = self.read_phylactery_knowledge()?.ok_or_else(|| {
                    ReliquaryRuntimeHostError::Operation("Phylactery is unavailable".into())
                })?;
                state
                    .0
                    .into_iter()
                    .find(|memory| memory.id == memory_id)
                    .ok_or_else(|| {
                        ReliquaryRuntimeHostError::Operation(
                            "Knowledge Memory is unavailable".into(),
                        )
                    })
            }
        }
    }
}
