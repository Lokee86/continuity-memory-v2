#[path = "entity_store_read.rs"]
mod read;
#[path = "entity_store_validation.rs"]
mod validation;

use crate::entity_codec::{EntityVersion, encode_format, encode_record, encode_version};
use crate::entity_model::EntityRecord;
use crate::{Container, Entity, EntityDraft, EntityError, EntityId};
use sha2::{Digest, Sha256};
use std::collections::{BTreeSet, HashMap};
use validation::{normalize_draft, validate_record};

pub(crate) struct EntityStore {
    records: Vec<EntityRecord>,
    current: HashMap<EntityId, usize>,
    by_mutation: HashMap<String, usize>,
    by_surface: HashMap<String, BTreeSet<EntityId>>,
    next_entity_version: u64,
}

impl EntityStore {
    pub(crate) fn empty() -> Self {
        Self {
            records: Vec::new(),
            current: HashMap::new(),
            by_mutation: HashMap::new(),
            by_surface: HashMap::new(),
            next_entity_version: 1,
        }
    }

    pub(crate) fn initialize(&self, container: &mut Container) -> Result<(), EntityError> {
        container.append(&encode_format())?;
        Ok(())
    }

    pub(crate) fn publish(
        &mut self,
        container: &mut Container,
        id: Option<EntityId>,
        expected_revision: u64,
        mut draft: EntityDraft,
    ) -> Result<(Entity, bool), EntityError> {
        normalize_draft(&mut draft)?;
        if let Some(index) = self.by_mutation.get(&draft.mutation_id).copied() {
            let existing = resolve(&self.records[index]);
            return if same_draft(&existing, &draft) {
                Ok((existing, false))
            } else {
                Err(EntityError::MutationConflict)
            };
        }

        let id = id.unwrap_or_else(|| entity_id(&draft.mutation_id));
        let current = self.current.get(&id).map(|index| &self.records[*index]);
        let current_revision = current.map_or(0, |record| record.revision);
        if current_revision != expected_revision {
            return Err(EntityError::RevisionConflict);
        }
        if current.is_some_and(|record| record.created_at_ns != draft.created_at_ns) {
            return Err(EntityError::InvalidField("Entity created_at"));
        }

        let revision = expected_revision
            .checked_add(1)
            .ok_or(EntityError::VersionExhausted)?;
        let entity_version = self.next_entity_version;
        entity_version
            .checked_add(1)
            .ok_or(EntityError::VersionExhausted)?;

        let mut record = EntityRecord {
            id,
            revision,
            canonical_name: draft.canonical_name,
            aliases: draft.aliases,
            kind: draft.kind,
            summary: draft.summary,
            mutation_id: draft.mutation_id,
            created_at_ns: draft.created_at_ns,
            updated_at_ns: draft.updated_at_ns,
            global_version: 0,
            entity_version,
        };
        let record_ref = container.append(&encode_record(&record)?)?;
        let global_version = container.allocate_version()?;
        container.append(&encode_version(EntityVersion {
            global_version,
            entity_version,
            record: record_ref,
        }))?;
        record.global_version = global_version;
        self.insert_rebuilt(record.clone())?;
        Ok((resolve(&record), true))
    }

    pub(crate) fn insert_rebuilt(&mut self, record: EntityRecord) -> Result<(), EntityError> {
        if record.revision == 0
            || record.entity_version == 0
            || record.global_version == 0
            || record.entity_version != self.next_entity_version
        {
            return Err(EntityError::InvalidVersion);
        }
        if let Some(index) = self.by_mutation.get(&record.mutation_id).copied() {
            return if self.records[index] == record {
                Ok(())
            } else {
                Err(EntityError::MutationConflict)
            };
        }
        let current = self
            .current
            .get(&record.id)
            .map(|index| &self.records[*index]);
        let expected_revision = current.map_or(1, |record| record.revision + 1);
        if record.revision != expected_revision
            || current.is_some_and(|prior| prior.created_at_ns != record.created_at_ns)
        {
            return Err(EntityError::RevisionConflict);
        }
        validate_record(&record)?;

        if let Some(index) = self.current.get(&record.id).copied() {
            let prior = &self.records[index];
            for surface in entity_surfaces(&prior.canonical_name, &prior.aliases) {
                remove_surface(&mut self.by_surface, &surface, record.id);
            }
        }

        let index = self.records.len();
        self.by_mutation.insert(record.mutation_id.clone(), index);
        self.current.insert(record.id, index);
        for surface in entity_surfaces(&record.canonical_name, &record.aliases) {
            self.by_surface
                .entry(surface)
                .or_default()
                .insert(record.id);
        }
        self.records.push(record);
        self.next_entity_version = self
            .next_entity_version
            .checked_add(1)
            .ok_or(EntityError::VersionExhausted)?;
        Ok(())
    }
}

fn entity_id(mutation_id: &str) -> EntityId {
    let mut hash = Sha256::new();
    hash.update(b"reliquary-entity-id\0");
    hash.update((mutation_id.len() as u64).to_le_bytes());
    hash.update(mutation_id.as_bytes());
    EntityId(hash.finalize().into())
}

fn resolve(record: &EntityRecord) -> Entity {
    Entity {
        id: record.id,
        revision: record.revision,
        canonical_name: record.canonical_name.clone(),
        aliases: record.aliases.clone(),
        kind: record.kind.clone(),
        summary: record.summary.clone(),
        mutation_id: record.mutation_id.clone(),
        created_at_ns: record.created_at_ns,
        updated_at_ns: record.updated_at_ns,
        global_version: record.global_version,
        entity_version: record.entity_version,
    }
}

fn normalize_surface(value: &str) -> String {
    value.trim().to_lowercase()
}

fn entity_surfaces(canonical_name: &str, aliases: &[String]) -> BTreeSet<String> {
    std::iter::once(canonical_name)
        .chain(aliases.iter().map(String::as_str))
        .map(normalize_surface)
        .collect()
}

fn remove_surface(
    index: &mut HashMap<String, BTreeSet<EntityId>>,
    surface: &str,
    entity_id: EntityId,
) {
    if let Some(ids) = index.get_mut(surface) {
        ids.remove(&entity_id);
        if ids.is_empty() {
            index.remove(surface);
        }
    }
}

fn same_draft(entity: &Entity, draft: &EntityDraft) -> bool {
    entity.canonical_name == draft.canonical_name
        && entity.aliases == draft.aliases
        && entity.kind == draft.kind
        && entity.summary == draft.summary
        && entity.mutation_id == draft.mutation_id
        && entity.created_at_ns == draft.created_at_ns
        && entity.updated_at_ns == draft.updated_at_ns
}
