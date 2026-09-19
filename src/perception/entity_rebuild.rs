use crate::entity_codec::{decode_format, decode_record, decode_version};
use crate::entity_model::EntityRecord;
use crate::{EntityError, ObjectRef};
use std::collections::HashMap;

pub(crate) struct EntityOpenState {
    store: crate::entity_store::EntityStore,
    pending_records: HashMap<ObjectRef, EntityRecord>,
    format_seen: bool,
}

impl EntityOpenState {
    pub(crate) fn new() -> Self {
        Self {
            store: crate::entity_store::EntityStore::empty(),
            pending_records: HashMap::new(),
            format_seen: false,
        }
    }

    pub(crate) fn ingest(
        &mut self,
        chunk: ObjectRef,
        payload: &[u8],
        latest_global_version: u64,
    ) -> Result<(), EntityError> {
        if decode_format(payload) {
            if self.format_seen {
                return Err(EntityError::ConflictingFormat);
            }
            self.format_seen = true;
            return Ok(());
        }
        if let Some(record) = decode_record(payload)? {
            self.pending_records.insert(chunk, record);
            return Ok(());
        }
        if let Some(version) = decode_version(payload)? {
            if version.global_version == 0 || version.global_version > latest_global_version {
                return Err(EntityError::InvalidVersion);
            }
            let mut record = self
                .pending_records
                .remove(&version.record)
                .ok_or(EntityError::InvalidVersion)?;
            record.global_version = version.global_version;
            record.entity_version = version.entity_version;
            self.store.insert_rebuilt(record)?;
        }
        Ok(())
    }

    pub(crate) fn finish(self) -> Result<crate::entity_store::EntityStore, EntityError> {
        // Entity ownership was introduced after existing REL/PHY files existed.
        // Absence of the format marker therefore means an empty legacy Entity owner.
        Ok(self.store)
    }
}
