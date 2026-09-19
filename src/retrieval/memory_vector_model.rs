use crate::{CompatibilityProfileId, MemoryBodyId, PackedVectorId};

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct MemoryVectorId(pub [u8; 32]);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MemoryVectorLocation {
    pub set_id: MemoryVectorId,
    pub row: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MemoryVectorSet {
    pub id: MemoryVectorId,
    pub compatibility_profile_id: CompatibilityProfileId,
    pub packed_vector_id: PackedVectorId,
    pub memory_body_ids: Vec<MemoryBodyId>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MemoryVectorInfo {
    pub id: MemoryVectorId,
    pub compatibility_profile_id: CompatibilityProfileId,
    pub packed_vector_id: PackedVectorId,
    pub rows: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MemoryVectorStats {
    pub objects: usize,
    pub rows: u64,
    pub bindings: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MemoryVectorBuildResult {
    pub created_set: Option<MemoryVectorId>,
    pub embedded: usize,
    pub already_present: usize,
}
