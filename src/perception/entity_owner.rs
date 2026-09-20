use crate::{Cva, Entity, EntityDraft, EntityError, EntityId, EntityStats, Phylactery};

macro_rules! impl_owner {
    ($owner:ty) => {
        impl $owner {
            pub fn publish_entity(
                &mut self,
                id: Option<EntityId>,
                expected_revision: u64,
                draft: EntityDraft,
            ) -> Result<(Entity, bool), EntityError> {
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

            pub fn entity_version(&self) -> u64 {
                self.entities.entity_version()
            }

            pub fn entity_stats(&self) -> EntityStats {
                self.entities.stats()
            }
        }
    };
}

impl_owner!(Cva);
impl_owner!(Phylactery);
