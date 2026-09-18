use crate::{
    Cva, EntityId, EntityResolutionCompaction, EntityResolutionReason, MemoryEntityMentionKey,
    MemoryEntityResolution, MemoryError, MemoryId, Phylactery,
};

macro_rules! impl_owner {
    ($owner:ty) => {
        impl $owner {
            pub fn entity_resolution(
                &self,
                key: MemoryEntityMentionKey,
            ) -> Option<&MemoryEntityResolution> {
                self.entity_resolutions.get(key)
            }

            pub fn entity_resolutions_for_memory(
                &self,
                memory_id: MemoryId,
            ) -> Vec<MemoryEntityResolution> {
                self.entity_resolutions.for_memory(memory_id)
            }

            pub fn entity_resolution_retry_needed(
                &self,
                key: MemoryEntityMentionKey,
                candidate_set_fingerprint: [u8; 32],
                context_fingerprint: [u8; 32],
            ) -> bool {
                self.entity_resolutions.retry_needed(
                    key,
                    candidate_set_fingerprint,
                    context_fingerprint,
                )
            }

            pub fn put_entity_resolved(
                &mut self,
                key: MemoryEntityMentionKey,
                expected_revision: u32,
                entity_id: EntityId,
                reason: EntityResolutionReason,
                now_ns: i64,
            ) -> Result<bool, MemoryError> {
                self.entity_resolutions.put_resolved(
                    &mut self.container,
                    &self.memories,
                    &self.entities,
                    key,
                    expected_revision,
                    entity_id,
                    reason,
                    now_ns,
                )
            }

            pub fn put_entity_rejected(
                &mut self,
                key: MemoryEntityMentionKey,
                expected_revision: u32,
                reason: EntityResolutionReason,
                now_ns: i64,
            ) -> Result<bool, MemoryError> {
                self.entity_resolutions.put_rejected(
                    &mut self.container,
                    &self.memories,
                    key,
                    expected_revision,
                    reason,
                    now_ns,
                )
            }

            #[allow(clippy::too_many_arguments)]
            pub fn put_entity_unresolved(
                &mut self,
                key: MemoryEntityMentionKey,
                expected_revision: u32,
                candidate_entity_ids: Vec<EntityId>,
                reason: EntityResolutionReason,
                candidate_set_fingerprint: [u8; 32],
                context_fingerprint: [u8; 32],
                now_ns: i64,
            ) -> Result<bool, MemoryError> {
                self.entity_resolutions.put_unresolved(
                    &mut self.container,
                    &self.memories,
                    &self.entities,
                    key,
                    expected_revision,
                    candidate_entity_ids,
                    reason,
                    candidate_set_fingerprint,
                    context_fingerprint,
                    now_ns,
                )
            }

            pub fn compact_entity_resolutions(
                &mut self,
                now_ns: i64,
            ) -> Result<EntityResolutionCompaction, MemoryError> {
                self.entity_resolutions
                    .compact(&mut self.container, &self.memories, now_ns)
            }

            pub(crate) fn entity_resolution_records(&self) -> Vec<MemoryEntityResolution> {
                self.entity_resolutions.records()
            }

            pub(crate) fn import_entity_resolution(
                &mut self,
                value: MemoryEntityResolution,
            ) -> Result<(), MemoryError> {
                self.entity_resolutions.import_current(
                    &mut self.container,
                    &self.memories,
                    &self.entities,
                    value,
                )
            }
        }
    };
}

impl_owner!(Cva);
impl_owner!(Phylactery);
