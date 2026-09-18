#[path = "entity_resolution_store_lifecycle.rs"]
mod lifecycle;
#[path = "entity_resolution_store_write.rs"]
mod write;

use crate::entity_resolution_codec::{decode_resolution, encode_resolution, encode_tombstone};
use crate::entity_store::EntityStore;
use crate::memory_store::MemoryStore;
use crate::{
    Container, MAX_ENTITY_RESOLUTION_CANDIDATES, MemoryEntityMentionKey, MemoryEntityResolution,
    MemoryEntityResolutionStatus, MemoryError, MemoryId,
};
use std::collections::{HashMap, HashSet};

#[derive(Default)]
pub(crate) struct EntityResolutionStore {
    current: HashMap<MemoryEntityMentionKey, MemoryEntityResolution>,
    revisions: HashMap<MemoryEntityMentionKey, u32>,
    maintenance: HashSet<MemoryEntityMentionKey>,
}

impl EntityResolutionStore {
    pub(crate) fn ingest(&mut self, payload: &[u8]) -> Result<(), MemoryError> {
        let Some(decoded) = decode_resolution(payload)? else {
            return Ok(());
        };
        let expected = self
            .revisions
            .get(&decoded.key)
            .copied()
            .unwrap_or(0)
            .checked_add(1)
            .ok_or(MemoryError::VersionExhausted)?;
        if decoded.revision != expected {
            return Err(MemoryError::CorruptRecord(
                "Entity resolution revision sequence",
            ));
        }
        match decoded.status {
            Some(status) => {
                let value = MemoryEntityResolution {
                    key: decoded.key,
                    revision: decoded.revision,
                    updated_at_ns: decoded.updated_at_ns,
                    status,
                };
                validate_status(&value.status)?;
                self.revisions.insert(value.key, value.revision);
                self.track_maintenance(value.key, &value.status);
                self.current.insert(value.key, value);
            }
            None => {
                self.current.remove(&decoded.key);
                self.revisions.remove(&decoded.key);
                self.maintenance.remove(&decoded.key);
            }
        }
        Ok(())
    }

    pub(crate) fn validate(
        &self,
        memories: &MemoryStore,
        entities: &EntityStore,
    ) -> Result<(), MemoryError> {
        for value in self.current.values() {
            memories.validate_entity_mention_key(value.key)?;
            validate_status(&value.status)?;
            validate_entity_refs(&value.status, entities)?;
        }
        Ok(())
    }

    pub(crate) fn get(&self, key: MemoryEntityMentionKey) -> Option<&MemoryEntityResolution> {
        self.current.get(&key)
    }

    pub(crate) fn for_memory(&self, id: MemoryId) -> Vec<MemoryEntityResolution> {
        let mut values: Vec<_> = self
            .current
            .values()
            .filter(|value| value.key.memory_id == id)
            .cloned()
            .collect();
        values.sort_by_key(|value| mention_sort_key(value.key));
        values
    }

    pub(crate) fn records(&self) -> Vec<MemoryEntityResolution> {
        let mut values: Vec<_> = self.current.values().cloned().collect();
        values.sort_by_key(|value| mention_sort_key(value.key));
        values
    }

    pub(crate) fn retry_needed(
        &self,
        key: MemoryEntityMentionKey,
        candidate_fingerprint: [u8; 32],
        context_fingerprint: [u8; 32],
    ) -> bool {
        match self.current.get(&key).map(|value| &value.status) {
            None => true,
            Some(MemoryEntityResolutionStatus::Pending(value)) => {
                value.candidate_set_fingerprint != candidate_fingerprint
                    || value.context_fingerprint != context_fingerprint
            }
            Some(MemoryEntityResolutionStatus::Dormant(value)) => {
                value.candidate_set_fingerprint != candidate_fingerprint
                    || value.context_fingerprint != context_fingerprint
            }
            Some(
                MemoryEntityResolutionStatus::Resolved { .. }
                | MemoryEntityResolutionStatus::Rejected { .. },
            ) => false,
        }
    }

    pub(super) fn append(
        &mut self,
        container: &mut Container,
        key: MemoryEntityMentionKey,
        updated_at_ns: i64,
        status: MemoryEntityResolutionStatus,
    ) -> Result<bool, MemoryError> {
        validate_status(&status)?;
        let revision = self
            .current
            .get(&key)
            .map(|value| value.revision)
            .unwrap_or(0)
            .checked_add(1)
            .ok_or(MemoryError::VersionExhausted)?;
        let value = MemoryEntityResolution {
            key,
            revision,
            updated_at_ns,
            status,
        };
        container.append(&encode_resolution(&value)?)?;
        self.revisions.insert(key, revision);
        self.track_maintenance(key, &value.status);
        self.current.insert(key, value);
        Ok(true)
    }

    pub(super) fn tombstone(
        &mut self,
        container: &mut Container,
        key: MemoryEntityMentionKey,
        now_ns: i64,
    ) -> Result<bool, MemoryError> {
        let Some(current) = self.current.get(&key) else {
            return Ok(false);
        };
        let revision = current
            .revision
            .checked_add(1)
            .ok_or(MemoryError::VersionExhausted)?;
        container.append(&encode_tombstone(key, revision, now_ns)?)?;
        self.current.remove(&key);
        self.revisions.remove(&key);
        self.maintenance.remove(&key);
        Ok(true)
    }

    pub(crate) fn import_current(
        &mut self,
        container: &mut Container,
        memories: &MemoryStore,
        entities: &EntityStore,
        mut value: MemoryEntityResolution,
    ) -> Result<(), MemoryError> {
        memories.validate_entity_mention_key(value.key)?;
        validate_status(&value.status)?;
        validate_entity_refs(&value.status, entities)?;
        if let Some(existing) = self.current.get(&value.key) {
            return if existing.status == value.status {
                Ok(())
            } else {
                Err(MemoryError::RoutingMetadataConflict)
            };
        }
        value.revision = 1;
        container.append(&encode_resolution(&value)?)?;
        self.revisions.insert(value.key, 1);
        self.track_maintenance(value.key, &value.status);
        self.current.insert(value.key, value);
        Ok(())
    }

    fn track_maintenance(
        &mut self,
        key: MemoryEntityMentionKey,
        status: &MemoryEntityResolutionStatus,
    ) {
        if matches!(status, MemoryEntityResolutionStatus::Resolved { .. }) {
            self.maintenance.remove(&key);
        } else {
            self.maintenance.insert(key);
        }
    }
}

fn validate_entity_refs(
    status: &MemoryEntityResolutionStatus,
    entities: &EntityStore,
) -> Result<(), MemoryError> {
    let valid = match status {
        MemoryEntityResolutionStatus::Resolved { entity_id, .. } => entities.contains(*entity_id),
        MemoryEntityResolutionStatus::Pending(value) => value
            .candidate_entity_ids
            .iter()
            .all(|id| entities.contains(*id)),
        MemoryEntityResolutionStatus::Dormant(value) => value
            .candidate_entity_ids
            .iter()
            .all(|id| entities.contains(*id)),
        MemoryEntityResolutionStatus::Rejected { .. } => true,
    };
    if valid {
        Ok(())
    } else {
        Err(MemoryError::InvalidField(
            "Entity resolution Entity reference",
        ))
    }
}

fn mention_sort_key(key: MemoryEntityMentionKey) -> ([u8; 32], u8, u32, u32) {
    (
        key.memory_id.0,
        key.field.tag(),
        key.start_byte,
        key.end_byte,
    )
}

pub(super) fn validate_status(status: &MemoryEntityResolutionStatus) -> Result<(), MemoryError> {
    let (candidates, first, last, attempt, dormant) = match status {
        MemoryEntityResolutionStatus::Pending(value) => (
            &value.candidate_entity_ids,
            Some(value.first_seen_at_ns),
            Some(value.last_attempt_at_ns),
            Some(value.attempt_count),
            None,
        ),
        MemoryEntityResolutionStatus::Dormant(value) => (
            &value.candidate_entity_ids,
            Some(value.first_seen_at_ns),
            Some(value.last_attempt_at_ns),
            Some(value.attempt_count),
            Some(value.dormant_at_ns),
        ),
        _ => return Ok(()),
    };
    if candidates.len() > MAX_ENTITY_RESOLUTION_CANDIDATES
        || candidates.windows(2).any(|pair| pair[0] >= pair[1])
        || attempt == Some(0)
        || first > last
        || dormant.is_some_and(|value| Some(value) < last)
    {
        return Err(MemoryError::InvalidField("Entity resolution state"));
    }
    Ok(())
}
