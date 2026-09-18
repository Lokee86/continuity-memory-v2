use super::EntityResolutionStore;
use crate::memory_store::MemoryStore;
use crate::{
    Container, EntityId, EntityResolutionPending, EntityResolutionReason,
    MAX_ENTITY_RESOLUTION_CANDIDATES, MemoryEntityMentionKey, MemoryEntityResolutionStatus,
    MemoryError,
};

impl EntityResolutionStore {
    pub(crate) fn put_resolved(
        &mut self,
        container: &mut Container,
        memories: &MemoryStore,
        key: MemoryEntityMentionKey,
        expected_revision: u32,
        entity_id: EntityId,
        reason: EntityResolutionReason,
        now_ns: i64,
    ) -> Result<bool, MemoryError> {
        self.validate_write(memories, key, expected_revision)?;
        if let Some(current) = self.current.get(&key) {
            if current.status == (MemoryEntityResolutionStatus::Resolved { entity_id, reason }) {
                return Ok(false);
            }
            reject_terminal(&current.status)?;
        }
        self.append(
            container,
            key,
            now_ns,
            MemoryEntityResolutionStatus::Resolved { entity_id, reason },
        )
    }

    pub(crate) fn put_rejected(
        &mut self,
        container: &mut Container,
        memories: &MemoryStore,
        key: MemoryEntityMentionKey,
        expected_revision: u32,
        reason: EntityResolutionReason,
        now_ns: i64,
    ) -> Result<bool, MemoryError> {
        self.validate_write(memories, key, expected_revision)?;
        if let Some(current) = self.current.get(&key) {
            if current.status == (MemoryEntityResolutionStatus::Rejected { reason }) {
                return Ok(false);
            }
            reject_terminal(&current.status)?;
        }
        self.append(
            container,
            key,
            now_ns,
            MemoryEntityResolutionStatus::Rejected { reason },
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn put_unresolved(
        &mut self,
        container: &mut Container,
        memories: &MemoryStore,
        key: MemoryEntityMentionKey,
        expected_revision: u32,
        mut candidate_entity_ids: Vec<EntityId>,
        reason: EntityResolutionReason,
        candidate_set_fingerprint: [u8; 32],
        context_fingerprint: [u8; 32],
        now_ns: i64,
    ) -> Result<bool, MemoryError> {
        self.validate_write(memories, key, expected_revision)?;
        normalize_candidates(&mut candidate_entity_ids)?;
        let (first_seen_at_ns, attempt_count) = match self.current.get(&key) {
            None => (now_ns, 1),
            Some(current) => match &current.status {
                MemoryEntityResolutionStatus::Pending(value) => {
                    if same_evidence(
                        value.candidate_set_fingerprint,
                        value.context_fingerprint,
                        candidate_set_fingerprint,
                        context_fingerprint,
                    ) {
                        return Ok(false);
                    }
                    validate_retry_time(now_ns, value.last_attempt_at_ns)?;
                    (value.first_seen_at_ns, next_attempt(value.attempt_count)?)
                }
                MemoryEntityResolutionStatus::Dormant(value) => {
                    if same_evidence(
                        value.candidate_set_fingerprint,
                        value.context_fingerprint,
                        candidate_set_fingerprint,
                        context_fingerprint,
                    ) {
                        return Ok(false);
                    }
                    validate_retry_time(now_ns, value.last_attempt_at_ns)?;
                    (value.first_seen_at_ns, next_attempt(value.attempt_count)?)
                }
                status => {
                    reject_terminal(status)?;
                    unreachable!()
                }
            },
        };
        self.append(
            container,
            key,
            now_ns,
            MemoryEntityResolutionStatus::Pending(EntityResolutionPending {
                candidate_entity_ids,
                reason,
                candidate_set_fingerprint,
                context_fingerprint,
                first_seen_at_ns,
                last_attempt_at_ns: now_ns,
                attempt_count,
            }),
        )
    }

    fn validate_write(
        &self,
        memories: &MemoryStore,
        key: MemoryEntityMentionKey,
        expected_revision: u32,
    ) -> Result<(), MemoryError> {
        memories.validate_entity_mention_key(key)?;
        let current = self
            .current
            .get(&key)
            .map(|value| value.revision)
            .unwrap_or(0);
        if current != expected_revision {
            return Err(MemoryError::RevisionConflict);
        }
        Ok(())
    }
}

fn reject_terminal(status: &MemoryEntityResolutionStatus) -> Result<(), MemoryError> {
    if matches!(
        status,
        MemoryEntityResolutionStatus::Resolved { .. }
            | MemoryEntityResolutionStatus::Rejected { .. }
    ) {
        Err(MemoryError::InvalidField("terminal Entity resolution"))
    } else {
        Ok(())
    }
}

fn normalize_candidates(values: &mut Vec<EntityId>) -> Result<(), MemoryError> {
    values.sort();
    values.dedup();
    if values.len() > MAX_ENTITY_RESOLUTION_CANDIDATES {
        return Err(MemoryError::FieldTooLarge);
    }
    Ok(())
}

fn next_attempt(value: u16) -> Result<u16, MemoryError> {
    value.checked_add(1).ok_or(MemoryError::VersionExhausted)
}

fn validate_retry_time(now_ns: i64, last_ns: i64) -> Result<(), MemoryError> {
    if now_ns < last_ns {
        Err(MemoryError::InvalidField("Entity resolution timestamp"))
    } else {
        Ok(())
    }
}

fn same_evidence(
    old_candidates: [u8; 32],
    old_context: [u8; 32],
    new_candidates: [u8; 32],
    new_context: [u8; 32],
) -> bool {
    old_candidates == new_candidates && old_context == new_context
}
