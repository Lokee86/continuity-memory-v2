use super::{EntityStore, alias_surface_keys, normalize_surface, normalized_surface, resolve};
use crate::entity_identity_guard::{identity_guard_keys, single_token};
use crate::entity_model::EntityRecord;
use crate::{Entity, EntityError, EntityId, EntityStats};
use std::collections::BTreeSet;

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
        self.by_surface
            .get(&needle)
            .into_iter()
            .flat_map(|ids| ids.iter())
            .take(limit)
            .filter_map(|id| self.current.get(id))
            .map(|index| resolve(&self.records[*index]))
            .collect()
    }

    pub(crate) fn candidates_for_normalized_surface(
        &self,
        surface: &str,
        limit: usize,
    ) -> Vec<Entity> {
        let needle = normalized_surface(surface);
        if needle.is_empty() {
            return Vec::new();
        }
        self.by_normalized_surface
            .get(&needle)
            .into_iter()
            .flat_map(|ids| ids.iter())
            .take(limit)
            .filter_map(|id| self.current.get(id))
            .map(|index| resolve(&self.records[*index]))
            .collect()
    }

    pub(crate) fn candidates_for_alias_surface(&self, surface: &str, limit: usize) -> Vec<Entity> {
        let mut ids = std::collections::BTreeSet::new();
        for key in alias_surface_keys(surface) {
            if let Some(values) = self.by_alias_surface.get(&key) {
                for id in values {
                    ids.insert(*id);
                    if ids.len() >= limit {
                        break;
                    }
                }
            }
            if ids.len() >= limit {
                break;
            }
        }
        ids.into_iter()
            .filter_map(|id| self.current.get(&id))
            .map(|index| resolve(&self.records[*index]))
            .collect()
    }

    pub(crate) fn creation_conflicts(
        &self,
        surface: &str,
        kind: &str,
        limit: usize,
    ) -> Vec<Entity> {
        let mut qualifiers = BTreeSet::new();
        for index in self.current.values() {
            let entity = resolve(&self.records[*index]);
            if entity.kind == "programming_language" {
                for value in std::iter::once(entity.canonical_name.as_str())
                    .chain(entity.aliases.iter().map(String::as_str))
                {
                    if let Some(token) = single_token(value) {
                        qualifiers.insert(token);
                    }
                }
            }
        }
        let wanted = identity_guard_keys(surface, &qualifiers);
        if wanted.is_empty() {
            return Vec::new();
        }

        let mut matches = Vec::new();
        for index in self.current.values() {
            let entity = resolve(&self.records[*index]);
            if entity.kind != kind {
                continue;
            }
            let conflicts = std::iter::once(entity.canonical_name.as_str())
                .chain(entity.aliases.iter().map(String::as_str))
                .any(|candidate| {
                    identity_guard_keys(candidate, &qualifiers)
                        .iter()
                        .any(|key| wanted.contains(key))
                });
            if conflicts {
                matches.push(entity);
            }
        }
        matches.sort_by_key(|entity| entity.id);
        matches.truncate(limit);
        matches
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
