use crate::{
    EntityCandidateSet, EntityMaterialization, EntityResolutionOutcome, EntityResolverOutput,
    Memory, MemoryEntityMentionKey,
};

pub(crate) enum EntityResolutionPreparation {
    Complete(EntityResolutionOutcome),
    Ready(EntityResolutionPrepared),
}

pub(crate) struct EntityResolutionPrepared {
    pub(crate) owner_id: String,
    pub(crate) key: MemoryEntityMentionKey,
    pub(crate) memory: Memory,
    pub(crate) candidates: EntityCandidateSet,
    pub(crate) evidence: Vec<Vec<Memory>>,
    pub(crate) admission_context: Vec<Memory>,
    pub(crate) candidate_fingerprint: [u8; 32],
    pub(crate) context_fingerprint: [u8; 32],
    pub(crate) expected_resolution_revision: u32,
    pub(crate) memory_version: u64,
    pub(crate) entity_version: u64,
    pub(crate) graph_version: u64,
}

pub(crate) struct EntityResolutionEvaluation {
    pub(crate) output: EntityResolverOutput,
    pub(crate) materialization: Option<EntityMaterialization>,
}
