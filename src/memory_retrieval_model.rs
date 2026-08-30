use crate::memory_retrieval_index::CommunitySubcentroid;
use crate::{CommunityId, CompatibilityProfileId, MemoryId};
use std::collections::{HashMap, HashSet};

pub const DEFAULT_MEMORY_RETRIEVAL_COMMUNITIES: usize = 4;
pub const DEFAULT_MEMORY_RETRIEVAL_SUBCENTROIDS: usize = 4;
pub const DEFAULT_MEMORY_RETRIEVAL_SEEDS: usize = 4;
pub const DEFAULT_MEMORY_RETRIEVAL_BUDGET: usize = 32;
pub const DEFAULT_MEMORY_RETRIEVAL_MAX_DEPTH: usize = 3;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MemoryRetrievalMode {
    CommunityRouted,
    GlobalExact,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MemoryRetrievalConfig {
    pub mode: MemoryRetrievalMode,
    pub community_limit: usize,
    pub subcentroids_per_community: usize,
    pub seed_limit: usize,
    pub traversal_budget: usize,
    pub max_depth: usize,
}

impl Default for MemoryRetrievalConfig {
    fn default() -> Self {
        Self {
            mode: MemoryRetrievalMode::CommunityRouted,
            community_limit: DEFAULT_MEMORY_RETRIEVAL_COMMUNITIES,
            subcentroids_per_community: DEFAULT_MEMORY_RETRIEVAL_SUBCENTROIDS,
            seed_limit: DEFAULT_MEMORY_RETRIEVAL_SEEDS,
            traversal_budget: DEFAULT_MEMORY_RETRIEVAL_BUDGET,
            max_depth: DEFAULT_MEMORY_RETRIEVAL_MAX_DEPTH,
        }
    }
}

#[derive(Clone, Debug)]
pub struct MemoryRetrievalIndex {
    pub compatibility_profile_id: CompatibilityProfileId,
    pub memory_version: u64,
    pub graph_version: u64,
    pub community_generation: Option<u64>,
    pub vector_bindings: usize,
    pub subcentroids_per_community: usize,
    pub dimensions: usize,
    pub indexed_memories: usize,
    pub routing_vectors: usize,
    pub(crate) vectors: HashMap<MemoryId, Vec<f32>>,
    pub(crate) searchable: HashSet<MemoryId>,
    pub(crate) memberships: HashMap<MemoryId, CommunityId>,
    pub(crate) subcentroids: Vec<CommunitySubcentroid>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MemoryRetrievalHit {
    pub memory_id: MemoryId,
    pub score: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MemoryRetrievalResult {
    pub mode_used: MemoryRetrievalMode,
    pub used_global_fallback: bool,
    pub selected_communities: Vec<CommunityId>,
    pub seeds: Vec<MemoryRetrievalHit>,
    pub memories: Vec<MemoryId>,
    pub routing_vectors_scored: usize,
    pub memory_vectors_scored: usize,
    pub global_memory_vectors: usize,
}
