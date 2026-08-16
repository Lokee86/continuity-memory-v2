use super::*;
use std::collections::HashSet;

pub(crate) struct PreparedMemoryBatch {
    pub(crate) records: Vec<MemoryRecord>,
    pub(crate) existing: Vec<Memory>,
}

impl MemoryStore {
    pub(crate) fn stage_grouped_insomnia(
        &mut self,
        container: &mut Container,
        drafts: Vec<MemoryDraft>,
    ) -> Result<PreparedMemoryBatch, MemoryError> {
        let mut records = Vec::new();
        let mut existing = Vec::new();
        let mut seen_mutations = HashSet::new();
        for draft in drafts {
            validate_draft(&draft)?;
            if !seen_mutations.insert(draft.mutation_id.clone()) {
                return Err(MemoryError::MutationConflict);
            }
            if let Some(index) = self.by_mutation.get(&draft.mutation_id).copied() {
                let memory = self.resolve_record(container, &self.records[index])?;
                if !same_draft(&memory, &draft) {
                    return Err(MemoryError::MutationConflict);
                }
                existing.push(memory);
                continue;
            }
            let id = memory_id(&draft.mutation_id);
            if self.current.contains_key(&id) {
                return Err(MemoryError::RevisionConflict);
            }
            let body_id = self.put_body(container, &draft.title, &draft.content)?;
            let offset = u64::try_from(records.len()).map_err(|_| MemoryError::VersionExhausted)?;
            let memory_version = self
                .next_memory_version
                .checked_add(offset)
                .ok_or(MemoryError::VersionExhausted)?;
            let global_version = container.allocate_version()?;
            records.push(MemoryRecord {
                id,
                revision: 1,
                body_id,
                category: draft.category,
                memory_type: draft.memory_type,
                scope: draft.scope,
                lifecycle_state: draft.lifecycle_state,
                archived: draft.archived,
                superseded_by: draft.superseded_by,
                parent_id: draft.parent_id,
                source_node_id: draft.source_node_id,
                content_source_conversation_id: draft.content_source_conversation_id,
                content_source_node_id: draft.content_source_node_id,
                grounding_source_conversation_id: draft.grounding_source_conversation_id,
                grounding_source_node_id: draft.grounding_source_node_id,
                source_episode_id: draft.source_episode_id,
                mutation_id: draft.mutation_id,
                created_at_ns: draft.created_at_ns,
                updated_at_ns: draft.updated_at_ns,
                global_version,
                memory_version,
            });
        }
        Ok(PreparedMemoryBatch { records, existing })
    }

    pub(crate) fn apply_grouped_records(
        &mut self,
        container: &mut Container,
        records: &[MemoryRecord],
    ) -> Result<Vec<Memory>, MemoryError> {
        let mut created = Vec::with_capacity(records.len());
        for record in records {
            if !self.bodies.contains_key(&record.body_id) {
                return Err(MemoryError::MissingBody);
            }
            self.insert_rebuilt(record.clone())?;
            created.push(self.resolve_record(container, record)?);
        }
        Ok(created)
    }
}
