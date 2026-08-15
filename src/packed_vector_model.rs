use lodestone_packed::VectorSchema;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct PackedVectorId(pub [u8; 32]);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PackedVectorInfo {
    pub id: PackedVectorId,
    pub schema: VectorSchema,
    pub count: u64,
    pub byte_len: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PackedVectorStats {
    pub objects: usize,
    pub rows: u64,
    pub matrix_bytes: u64,
}
