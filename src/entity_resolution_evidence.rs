use crate::{Cva, EntityCandidateSet, EntityResolverError, Memory, MemoryId, Phylactery};

pub(super) trait EvidenceOwner {
    fn entity_memory_ids(&self, entity_id: crate::EntityId) -> Vec<MemoryId>;
    fn load_memory(&mut self, memory_id: MemoryId) -> Result<Memory, crate::MemoryError>;
}

impl EvidenceOwner for Cva {
    fn entity_memory_ids(&self, entity_id: crate::EntityId) -> Vec<MemoryId> {
        self.memories_for_entity(entity_id)
    }

    fn load_memory(&mut self, memory_id: MemoryId) -> Result<Memory, crate::MemoryError> {
        self.memory(memory_id)
    }
}

impl EvidenceOwner for Phylactery {
    fn entity_memory_ids(&self, entity_id: crate::EntityId) -> Vec<MemoryId> {
        self.memories_for_entity(entity_id)
    }

    fn load_memory(&mut self, memory_id: MemoryId) -> Result<Memory, crate::MemoryError> {
        self.memory(memory_id)
    }
}

pub(super) fn hydrate_admission_context(
    owner: &mut impl EvidenceOwner,
    set: &EntityCandidateSet,
    source_id: MemoryId,
) -> Result<Vec<Memory>, EntityResolverError> {
    let mut context = Vec::new();
    for id in &set.admission_context_memory_ids {
        if *id == source_id {
            continue;
        }
        let memory = owner.load_memory(*id)?;
        if !memory.archived {
            context.push(memory);
        }
    }
    Ok(context)
}

pub(super) fn hydrate_evidence(
    owner: &mut impl EvidenceOwner,
    set: &EntityCandidateSet,
    source_id: MemoryId,
) -> Result<Vec<Vec<Memory>>, EntityResolverError> {
    let mut all = Vec::with_capacity(set.candidates.len());
    for candidate in &set.candidates {
        let mut ids = Vec::new();
        for id in owner.entity_memory_ids(candidate.entity.id) {
            if id != source_id && !ids.contains(&id) {
                ids.push(id);
            }
        }
        for id in &candidate.supporting_memory_ids {
            if *id != source_id && !ids.contains(id) {
                ids.push(*id);
            }
        }
        ids.truncate(crate::MAX_ENTITY_RESOLVER_EVIDENCE_MEMORIES);

        let mut evidence = Vec::with_capacity(ids.len());
        for id in ids {
            let memory = owner.load_memory(id)?;
            if !memory.archived {
                evidence.push(memory);
            }
        }
        all.push(evidence);
    }
    Ok(all)
}
