use super::{EntityStore, resolve};
use crate::entity_model::EntityRecord;
use crate::{Entity, EntityError, EntityId, EntityStats};

impl EntityStore {
    pub(crate) fn entity(&self, id: EntityId) -> Result<Entity, EntityError> {
        self.current
            .get(&id)
            .map(|index| resolve(&self.records[*index]))
            .ok_or(EntityError::MissingEntity)
    }

    pub(crate) fn contains(&self, id: EntityId) -> bool {
        self.current.contains_key(&id)
    }

    pub(crate) fn current(&self) -> Vec<Entity> {
        let mut values: Vec<_> = self
            .current
            .values()
            .map(|index| resolve(&self.records[*index]))
            .collect();
        values.sort_by_key(|value| value.id);
        values
    }

    pub(crate) fn candidates_for_surface(&self, surface: &str, limit: usize) -> Vec<Entity> {
        let needle = normalize_surface(surface);
        let mut values: Vec<_> = self
            .current()
            .into_iter()
            .filter(|entity| {
                normalize_surface(&entity.canonical_name) == needle
                    || entity
                        .aliases
                        .iter()
                        .any(|alias| normalize_surface(alias) == needle)
            })
            .collect();
        values.sort_by_key(|value| value.id);
        values.truncate(limit);
        values
    }

    pub(crate) fn records(&self) -> &[EntityRecord] {
        &self.records
    }

    pub(crate) fn entity_version(&self) -> u64 {
        self.next_entity_version.saturating_sub(1)
    }

    pub(crate) fn stats(&self) -> EntityStats {
        EntityStats {
            entities: self.current.len(),
            revisions: self.records.len(),
            entity_version: self.entity_version(),
        }
    }
}

fn normalize_surface(value: &str) -> String {
    value.trim().to_lowercase()
}
