use super::{ReliquaryRuntimeHost, ReliquaryRuntimeHostError, operation};
use crate::{GraphRelationChange, GraphRelationKind, GraphRelationOrigin, MemoryId};

impl ReliquaryRuntimeHost {
    pub fn mutate_reliquary_knowledge_relation(
        &self,
        source: MemoryId,
        target: MemoryId,
        old_kind: Option<GraphRelationKind>,
        new_kind: Option<GraphRelationKind>,
    ) -> Result<(), ReliquaryRuntimeHostError> {
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
        let changes = relation_changes(source, target, old_kind, new_kind)?;
        let graph_version = runtime.cva.graph_version();
        runtime
            .cva
            .set_memory_relations_with_origin(&changes, GraphRelationOrigin::User, graph_version)
            .map_err(operation)?;
        runtime.cva.sync().map_err(operation)?;
        drop(runtime);
        self.wake()?;
        Ok(())
    }

    pub fn mutate_phylactery_knowledge_relation(
        &self,
        source: MemoryId,
        target: MemoryId,
        old_kind: Option<GraphRelationKind>,
        new_kind: Option<GraphRelationKind>,
    ) -> Result<(), ReliquaryRuntimeHostError> {
        let mut slot = self
            .active_execution()?
            .phylactery
            .lock()
            .map_err(|_| ReliquaryRuntimeHostError::LockPoisoned)?;
        let phylactery = slot.as_mut().ok_or_else(|| {
            ReliquaryRuntimeHostError::Operation("Phylactery is unavailable".into())
        })?;
        let changes = relation_changes(source, target, old_kind, new_kind)?;
        let graph_version = phylactery.graph_version();
        phylactery
            .set_memory_relations_with_origin(&changes, GraphRelationOrigin::User, graph_version)
            .map_err(operation)?;
        phylactery.sync().map_err(operation)?;
        drop(slot);
        self.wake()?;
        Ok(())
    }
}

fn relation_changes(
    source: MemoryId,
    target: MemoryId,
    old_kind: Option<GraphRelationKind>,
    new_kind: Option<GraphRelationKind>,
) -> Result<Vec<GraphRelationChange>, ReliquaryRuntimeHostError> {
    match (old_kind, new_kind) {
        (None, None) => Err(ReliquaryRuntimeHostError::Operation(
            "Knowledge relation mutation requires an old or new relation kind".into(),
        )),
        (Some(kind), Some(next)) if kind == next => Ok(vec![GraphRelationChange {
            source,
            target,
            kind,
            active: true,
        }]),
        (old_kind, new_kind) => {
            let mut changes = Vec::with_capacity(2);
            if let Some(kind) = old_kind {
                changes.push(GraphRelationChange {
                    source,
                    target,
                    kind,
                    active: false,
                });
            }
            if let Some(kind) = new_kind {
                changes.push(GraphRelationChange {
                    source,
                    target,
                    kind,
                    active: true,
                });
            }
            Ok(changes)
        }
    }
}
