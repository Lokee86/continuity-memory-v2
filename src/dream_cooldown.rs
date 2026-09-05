use crate::dream_source_time::{memory_source_timestamp_ns, reliquary_source_timestamp_ns};
use crate::memory_store::MemoryStore;
use crate::{Container, Cva, Memory, MemoryError, MemoryId, Phylactery};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

const DREAM_COOLDOWN_MAGIC: [u8; 8] = *b"CVADREM1";
pub const DEFAULT_DREAM_REPROCESS_COOLDOWN_NS: i64 = 30 * 24 * 60 * 60 * 1_000_000_000;

#[derive(Default)]
pub(crate) struct DreamCooldownStore {
    epochs: HashMap<MemoryId, u64>,
}

impl DreamCooldownStore {
    pub(crate) fn ingest(&mut self, payload: &[u8]) -> Result<(), MemoryError> {
        if payload.len() < 8 || payload[..8] != DREAM_COOLDOWN_MAGIC {
            return Ok(());
        }
        if payload.len() != 48 {
            return Err(MemoryError::CorruptRecord("invalid Dream cooldown record"));
        }
        let id = MemoryId(payload[8..40].try_into().unwrap());
        let epoch = u64::from_le_bytes(payload[40..48].try_into().unwrap());
        self.epochs
            .entry(id)
            .and_modify(|current| *current = (*current).max(epoch))
            .or_insert(epoch);
        Ok(())
    }

    pub(crate) fn epoch(&self, id: MemoryId) -> Option<u64> {
        self.epochs.get(&id).copied()
    }

    pub(crate) fn records(&self) -> Vec<(MemoryId, u64)> {
        let mut records: Vec<_> = self
            .epochs
            .iter()
            .map(|(id, epoch)| (*id, *epoch))
            .collect();
        records.sort_by_key(|(id, _)| id.0);
        records
    }

    pub(crate) fn validate(&self, memories: &MemoryStore) -> Result<(), MemoryError> {
        if self
            .epochs
            .keys()
            .all(|memory_id| memories.contains_memory(*memory_id))
        {
            Ok(())
        } else {
            Err(MemoryError::CorruptRecord(
                "Dream cooldown references a missing Memory",
            ))
        }
    }

    pub(crate) fn put(
        &mut self,
        container: &mut Container,
        id: MemoryId,
        epoch: u64,
    ) -> Result<bool, MemoryError> {
        if self.epoch(id).is_some_and(|current| current >= epoch) {
            return Ok(false);
        }
        let mut payload = Vec::with_capacity(48);
        payload.extend_from_slice(&DREAM_COOLDOWN_MAGIC);
        payload.extend_from_slice(&id.0);
        payload.extend_from_slice(&epoch.to_le_bytes());
        container.append(&payload)?;
        self.epochs.insert(id, epoch);
        Ok(true)
    }
}

pub(crate) fn dream_epoch(source_time_ns: i64, now_ns: i64) -> u64 {
    let elapsed = (now_ns as i128 - source_time_ns as i128).max(0);
    let epoch = elapsed / DEFAULT_DREAM_REPROCESS_COOLDOWN_NS as i128;
    epoch.min(u64::MAX as i128) as u64
}

pub(crate) fn eligible_dream_epoch(
    memory: &Memory,
    source_time_ns: Option<i64>,
    last_processed_epoch: Option<u64>,
    now_ns: i64,
) -> Option<u64> {
    if memory.archived {
        return None;
    }
    if memory.lifecycle_state == "extracted" {
        return Some(
            source_time_ns
                .map(|source_time| dream_epoch(source_time, now_ns))
                .unwrap_or(0),
        );
    }
    let source_time_ns = source_time_ns?;
    let current_epoch = dream_epoch(source_time_ns, now_ns);
    let satisfied_epoch =
        last_processed_epoch.unwrap_or_else(|| dream_epoch(source_time_ns, memory.updated_at_ns));
    (current_epoch > satisfied_epoch).then_some(current_epoch)
}

pub(crate) fn unix_now_ns() -> i64 {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    nanos.min(i64::MAX as u128) as i64
}

impl Cva {
    pub(crate) fn dream_eligible_epoch(
        &mut self,
        id: MemoryId,
        now_ns: i64,
    ) -> Result<Option<u64>, MemoryError> {
        let memory = self.memory(id)?;
        let source_time = reliquary_source_timestamp_ns(&self.archive, &memory);
        Ok(eligible_dream_epoch(
            &memory,
            source_time,
            self.dream_cooldowns.epoch(id),
            now_ns,
        ))
    }

    pub(crate) fn mark_dream_epoch(
        &mut self,
        id: MemoryId,
        epoch: u64,
    ) -> Result<bool, MemoryError> {
        if !self.memories.contains_memory(id) {
            return Err(MemoryError::MissingMemory);
        }
        self.dream_cooldowns.put(&mut self.container, id, epoch)
    }

    pub(crate) fn dream_cooldown_records(&self) -> Vec<(MemoryId, u64)> {
        self.dream_cooldowns.records()
    }
}

impl Phylactery {
    pub(crate) fn dream_eligible_epoch(
        &mut self,
        id: MemoryId,
        now_ns: i64,
    ) -> Result<Option<u64>, MemoryError> {
        let memory = self.memory(id)?;
        Ok(eligible_dream_epoch(
            &memory,
            memory_source_timestamp_ns(&memory),
            self.dream_cooldowns.epoch(id),
            now_ns,
        ))
    }

    pub(crate) fn mark_dream_epoch(
        &mut self,
        id: MemoryId,
        epoch: u64,
    ) -> Result<bool, MemoryError> {
        if !self.memories.contains_memory(id) {
            return Err(MemoryError::MissingMemory);
        }
        self.dream_cooldowns.put(&mut self.container, id, epoch)
    }
}
