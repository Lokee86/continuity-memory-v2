use crate::{Cva, Entity, EntityDraft, EntityError, EntityId, EntityStats, Phylactery};

macro_rules! impl_owner {
    ($owner:ty, $reject_principal:expr) => {
        impl $owner {
            pub fn publish_entity(
                &mut self,
                id: Option<EntityId>,
                expected_revision: u64,
                draft: EntityDraft,
            ) -> Result<(Entity, bool), EntityError> {
                if $reject_principal
                    && draft.kind.trim() == crate::entity_principal::PRINCIPAL_ENTITY_KIND
                {
                    return Err(EntityError::InvalidField("Phylactery principal Entity"));
                }
                self.entities
                    .publish(&mut self.container, id, expected_revision, draft)
            }

            pub fn entity(&self, id: EntityId) -> Result<Entity, EntityError> {
                self.entities.entity(id)
            }

            pub fn entities(&self) -> Vec<Entity> {
                self.entities.current()
            }

            pub fn entity_candidates_for_surface(
                &self,
                surface: &str,
                limit: usize,
            ) -> Vec<Entity> {
                self.entities.candidates_for_surface(surface, limit)
            }

            pub fn entity_candidates_for_normalized_surface(
                &self,
                surface: &str,
                limit: usize,
            ) -> Vec<Entity> {
                self.entities
                    .candidates_for_normalized_surface(surface, limit)
            }

            pub fn entity_candidates_for_alias_surface(
                &self,
                surface: &str,
                limit: usize,
            ) -> Vec<Entity> {
                self.entities.candidates_for_alias_surface(surface, limit)
            }

            pub(crate) fn entity_creation_conflicts(
                &self,
                surface: &str,
                kind: &str,
                limit: usize,
            ) -> Vec<Entity> {
                self.entities.creation_conflicts(surface, kind, limit)
            }

            pub fn retired_entity_replacement(&self, id: EntityId) -> Option<EntityId> {
                self.entities.retired_replacement(id)
            }

            pub fn entity_version(&self) -> u64 {
                self.entities.entity_version()
            }

            pub fn entity_stats(&self) -> EntityStats {
                self.entities.stats()
            }
        }
    };
}

impl_owner!(Cva, false);
impl_owner!(Phylactery, true);
