use crate::{FragmentId, PackedVectorId};

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct ArchiveVectorId(pub [u8; 32]);

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ArchiveVectorSet {
    pub id: ArchiveVectorId,
    pub packed_vector_id: PackedVectorId,
    pub fragment_ids: Vec<FragmentId>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ArchiveVectorInfo {
    pub id: ArchiveVectorId,
    pub packed_vector_id: PackedVectorId,
    pub rows: u64,
    pub max_fragment_archive_version: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ArchiveVectorStats {
    pub objects: usize,
    pub rows: u64,
}
