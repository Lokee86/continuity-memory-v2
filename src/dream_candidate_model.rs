use crate::{DreamTemporalAnalysis, DreamTemporalMatch, GraphRelation, Memory, MemoryBodyId};

pub const DEFAULT_DREAM_CANDIDATE_LIMIT: usize = 12;
pub const DEFAULT_DREAM_SEMANTIC_LIMIT: usize = 24;
pub const DEFAULT_DREAM_PRIOR_SEMANTIC_QUOTA: usize = 3;
pub const DEFAULT_DREAM_LEXICAL_LIMIT: usize = 8;
pub const DEFAULT_DREAM_TEMPORAL_LIMIT: usize = 8;
pub const MAX_DREAM_CANDIDATE_LIMIT: usize = 128;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DreamCandidateConfig {
    pub limit: usize,
    pub semantic_limit: usize,
    pub prior_semantic_quota: usize,
    pub lexical_limit: usize,
    pub temporal_limit: usize,
}

impl Default for DreamCandidateConfig {
    fn default() -> Self {
        Self {
            limit: DEFAULT_DREAM_CANDIDATE_LIMIT,
            semantic_limit: DEFAULT_DREAM_SEMANTIC_LIMIT,
            prior_semantic_quota: DEFAULT_DREAM_PRIOR_SEMANTIC_QUOTA,
            lexical_limit: DEFAULT_DREAM_LEXICAL_LIMIT,
            temporal_limit: DEFAULT_DREAM_TEMPORAL_LIMIT,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct DreamMemoryContext {
    pub memory: Memory,
    pub body_id: MemoryBodyId,
    pub source_timestamp_ns: Option<i64>,
    pub graph_relations: Vec<GraphRelation>,
    pub temporal: DreamTemporalAnalysis,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DreamCandidate {
    pub context: DreamMemoryContext,
    pub semantic_score: Option<f64>,
    pub semantic_rank: Option<usize>,
    pub prior_semantic_rank: Option<usize>,
    pub lexical_score: f64,
    pub lexical_rank: Option<usize>,
    pub temporal_score: f64,
    pub temporal_rank: Option<usize>,
    pub temporal_matches: Vec<DreamTemporalMatch>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DreamCandidateSet {
    pub source: DreamMemoryContext,
    pub candidates: Vec<DreamCandidate>,
}
