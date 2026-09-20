use crate::{
    EntityCandidateSet, EntityDraft, EntityMaterialization, EntityResolutionDecision,
    EntityResolutionOutcome, Memory, MemoryEntityMentionKey, MemoryEntityResolutionStatus,
};
use sha2::{Digest, Sha256};

pub(super) fn bootstrap_entity_draft(
    memory: &Memory,
    set: &EntityCandidateSet,
    materialization: EntityMaterialization,
) -> EntityDraft {
    EntityDraft {
        canonical_name: set.mention.text.clone(),
        aliases: Vec::new(),
        kind: materialization.kind,
        summary: materialization.summary,
        mutation_id: bootstrap_mutation_id(set.key),
        created_at_ns: memory.created_at_ns,
        updated_at_ns: memory.created_at_ns,
    }
}

pub(super) fn candidate_fingerprint(set: &EntityCandidateSet) -> [u8; 32] {
    let mut hash = Sha256::new();
    hash.update(b"reliquary-entity-candidate-set-v2\0");
    for candidate in &set.candidates {
        hash.update(candidate.entity.id.0);
        hash.update(candidate.entity.revision.to_le_bytes());
        hash.update([u8::from(candidate.exact_surface)]);
        hash.update([u8::from(candidate.normalized_surface)]);
        hash.update([u8::from(candidate.alias_surface)]);
        hash.update([u8::from(candidate.source_association)]);
        hash.update(candidate.lexical_memory_hits.to_le_bytes());
        hash.update(candidate.graph_neighbor_hits.to_le_bytes());
        hash.update(candidate.best_lexical_score.to_bits().to_le_bytes());
        for memory_id in &candidate.supporting_memory_ids {
            hash.update(memory_id.0);
        }
    }
    hash.finalize().into()
}

pub(super) fn context_fingerprint(
    memory: &Memory,
    set: &EntityCandidateSet,
    evidence: &[Vec<Memory>],
    admission_context: &[Memory],
) -> [u8; 32] {
    let mut hash = Sha256::new();
    hash.update(b"reliquary-entity-resolution-context-v1\0");
    hash.update(memory.id.0);
    hash.update(set.key.start_byte.to_le_bytes());
    hash.update(set.key.end_byte.to_le_bytes());
    hash.update(set.mention.text.as_bytes());
    for memories in evidence {
        for value in memories {
            hash.update(value.id.0);
            hash.update(value.revision.to_le_bytes());
        }
    }
    for value in admission_context {
        hash.update(value.id.0);
        hash.update(value.revision.to_le_bytes());
    }
    hash.finalize().into()
}

pub(super) fn terminal_outcome(
    key: MemoryEntityMentionKey,
    value: Option<&crate::MemoryEntityResolution>,
) -> Option<EntityResolutionOutcome> {
    let value = value?;
    match value.status {
        MemoryEntityResolutionStatus::Resolved { entity_id, reason } => {
            Some(EntityResolutionOutcome {
                key,
                decision: EntityResolutionDecision::ResolveExisting(entity_id),
                reason,
                entity_id: Some(entity_id),
                entity_created: false,
                association_changed: false,
                resolution_changed: false,
            })
        }
        MemoryEntityResolutionStatus::Rejected { reason } => Some(EntityResolutionOutcome {
            key,
            decision: EntityResolutionDecision::Reject,
            reason,
            entity_id: None,
            entity_created: false,
            association_changed: false,
            resolution_changed: false,
        }),
        _ => None,
    }
}

pub(super) fn status_reason(
    status: &MemoryEntityResolutionStatus,
) -> crate::EntityResolutionReason {
    match status {
        MemoryEntityResolutionStatus::Resolved { reason, .. }
        | MemoryEntityResolutionStatus::Rejected { reason } => *reason,
        MemoryEntityResolutionStatus::Pending(value) => value.reason,
        MemoryEntityResolutionStatus::Dormant(value) => value.reason,
    }
}

fn bootstrap_mutation_id(key: MemoryEntityMentionKey) -> String {
    format!(
        "perception-entity-v1:{}:{}:{}:{}",
        hex(&key.memory_id.0),
        key.field.tag(),
        key.start_byte,
        key.end_byte
    )
}

fn hex(bytes: &[u8; 32]) -> String {
    let mut out = String::with_capacity(64);
    for byte in bytes {
        use std::fmt::Write;
        let _ = write!(&mut out, "{byte:02x}");
    }
    out
}
