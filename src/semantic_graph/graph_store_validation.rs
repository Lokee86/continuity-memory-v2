use crate::entity_store::EntityStore;
use crate::memory_store::MemoryStore;
use crate::{
    GraphError, GraphRelationChange, GraphRelationOrigin, SemanticGraphRelationChange,
    SemanticGraphRelationKind, SemanticNodeKind, SemanticNodeRef,
};

pub(super) fn validate_memory_change(
    memories: &MemoryStore,
    change: GraphRelationChange,
    origin: GraphRelationOrigin,
) -> Result<(), GraphError> {
    if change.source == change.target {
        return Err(GraphError::SelfRelation);
    }
    for memory_id in [change.source, change.target] {
        if !memories.contains_memory(memory_id) {
            return Err(GraphError::MissingMemory(memory_id));
        }
    }
    if origin == GraphRelationOrigin::Perception {
        return Err(GraphError::InvalidRelationOrigin);
    }
    Ok(())
}

pub(super) fn validate_change(
    memories: &MemoryStore,
    entities: &EntityStore,
    change: SemanticGraphRelationChange,
    origin: GraphRelationOrigin,
) -> Result<(), GraphError> {
    validate_relation_contract(change.source, change.target, change.kind, origin)?;
    validate_node(memories, entities, change.source)?;
    validate_node(memories, entities, change.target)?;
    if let Some(entity_id) = change.target.as_entity() {
        let entity = entities
            .entity(entity_id)
            .map_err(|_| GraphError::MissingEntity(entity_id))?;
        match change.kind {
            SemanticGraphRelationKind::EntityAssociation
                if entity.kind == crate::entity_principal::PRINCIPAL_ENTITY_KIND =>
            {
                return Err(GraphError::InvalidRelationShape);
            }
            SemanticGraphRelationKind::PrincipalAssociation
                if entity.kind != crate::entity_principal::PRINCIPAL_ENTITY_KIND =>
            {
                return Err(GraphError::InvalidRelationShape);
            }
            _ => {}
        }
    }
    Ok(())
}

pub(super) fn validate_relation_contract(
    source: SemanticNodeRef,
    target: SemanticNodeRef,
    kind: SemanticGraphRelationKind,
    origin: GraphRelationOrigin,
) -> Result<(), GraphError> {
    if source == target {
        return Err(GraphError::SelfRelation);
    }
    match kind {
        SemanticGraphRelationKind::Memory(_) => {
            if source.kind != SemanticNodeKind::Memory || target.kind != SemanticNodeKind::Memory {
                return Err(GraphError::InvalidRelationShape);
            }
            if origin == GraphRelationOrigin::Perception {
                return Err(GraphError::InvalidRelationOrigin);
            }
        }
        SemanticGraphRelationKind::EntityAssociation
        | SemanticGraphRelationKind::PrincipalAssociation => {
            if source.kind != SemanticNodeKind::Memory || target.kind != SemanticNodeKind::Entity {
                return Err(GraphError::InvalidRelationShape);
            }
            if origin == GraphRelationOrigin::Dream {
                return Err(GraphError::InvalidRelationOrigin);
            }
        }
    }
    Ok(())
}

pub(super) fn validate_node(
    memories: &MemoryStore,
    entities: &EntityStore,
    node: SemanticNodeRef,
) -> Result<(), GraphError> {
    match node.kind {
        SemanticNodeKind::Memory => {
            let memory_id = node.as_memory().expect("Memory node decodes");
            if memories.contains_memory(memory_id) {
                Ok(())
            } else {
                Err(GraphError::MissingMemory(memory_id))
            }
        }
        SemanticNodeKind::Entity => {
            let entity_id = node.as_entity().expect("Entity node decodes");
            if entities.contains(entity_id) {
                Ok(())
            } else {
                Err(GraphError::MissingEntity(entity_id))
            }
        }
        SemanticNodeKind::Observation => Err(GraphError::UnsupportedSemanticNode(node)),
    }
}

pub(super) fn validate_stored_node(
    memories: &MemoryStore,
    entities: &EntityStore,
    node: SemanticNodeRef,
) -> Result<(), GraphError> {
    match node.kind {
        SemanticNodeKind::Entity => {
            let entity_id = node.as_entity().expect("Entity node decodes");
            if entities.contains(entity_id) || entities.retired_replacement(entity_id).is_some() {
                Ok(())
            } else {
                Err(GraphError::MissingEntity(entity_id))
            }
        }
        _ => validate_node(memories, entities, node),
    }
}
