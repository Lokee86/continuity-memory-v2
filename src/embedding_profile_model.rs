use crate::VectorNormalization;

pub const EMBEDDING_PROBE_SUITE_VERSION: u32 = 1;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct EmbeddingProfileId(pub [u8; 32]);

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EmbeddingProfile {
    pub id: EmbeddingProfileId,
    pub provider: String,
    pub model: String,
    pub revision: String,
    pub dimensions: u32,
    pub normalization: VectorNormalization,
    pub probe_suite_version: u32,
    pub behavior_fingerprint: [u8; 32],
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EmbeddingProfileStats {
    pub profiles: usize,
}
