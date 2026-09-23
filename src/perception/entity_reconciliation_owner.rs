use crate::entity_reconciliation::ReconciliationOwner;
use crate::entity_reconciliation_support::exact_entity_for_surface;
use crate::{
    Cva, EntityCandidateConfig, EntityReconciliationError, EntityResolutionEngine, GeneralEndpoint,
    Memory, MemoryEntityResolutionStatus, Phylactery,
};

macro_rules! impl_owner {
    ($owner:ty) => {
        impl ReconciliationOwner for $owner {
            fn support_memories(
                &mut self,
                entity_id: crate::EntityId,
            ) -> Result<Vec<Memory>, EntityReconciliationError> {
                let ids = self.memories_for_entity(entity_id);
                ids.into_iter()
                    .take(3)
                    .map(|id| {
                        self.memory(id)
                            .map_err(|error| EntityReconciliationError::Operation(error.to_string()))
                    })
                    .collect()
            }

            fn reconsider_rejections<E: GeneralEndpoint>(
                &mut self,
                endpoint: &E,
                timestamp: &mut i64,
            ) -> Result<usize, EntityReconciliationError> {
                let entities = self.entities();
                let states = self.entity_resolutions();
                let engine = EntityResolutionEngine::new(endpoint);
                let mut changed = 0usize;

                for state in states {
                    let MemoryEntityResolutionStatus::Rejected { reason } = state.status else {
                        continue;
                    };
                    let Some(metadata) = self.memory_routing_metadata(state.key.memory_id) else {
                        continue;
                    };
                    let Some(mention) = metadata.entity_mentions.iter().find(|mention| {
                        mention.field == state.key.field
                            && mention.start_byte == state.key.start_byte
                            && mention.end_byte == state.key.end_byte
                    }) else {
                        continue;
                    };
                    let Some(entity) = exact_entity_for_surface(&entities, &mention.text) else {
                        continue;
                    };
                    if entity.created_at_ns <= state.updated_at_ns {
                        continue;
                    }

                    *timestamp = timestamp.saturating_add(1);
                    self.reset_entity_resolution(state.key, *timestamp)
                        .map_err(|error| EntityReconciliationError::Operation(error.to_string()))?;
                    *timestamp = timestamp.saturating_add(1);
                    match self.resolve_entity_mention_with_engine(
                        &engine,
                        state.key,
                        EntityCandidateConfig::default(),
                        *timestamp,
                    ) {
                        Ok(_) => changed += 1,
                        Err(error) => {
                            *timestamp = timestamp.saturating_add(1);
                            self.put_entity_rejected(state.key, 0, reason, *timestamp)
                                .map_err(|restore| {
                                    EntityReconciliationError::Operation(format!(
                                        "resolver failed ({error}); failed to restore rejection: {restore}"
                                    ))
                                })?;
                            return Err(error.into());
                        }
                    }
                }
                Ok(changed)
            }
        }
    };
}

impl_owner!(Cva);
impl_owner!(Phylactery);
