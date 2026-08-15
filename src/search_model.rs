use crate::{Fragment, VectorGenerationId};

pub const DEFAULT_LEXICAL_WEIGHT: f64 = 0.45;
pub const DEFAULT_SEMANTIC_WEIGHT: f64 = 0.55;
pub const DEFAULT_SEARCH_CANDIDATE_LIMIT: usize = 30;
pub const DEFAULT_SEARCH_RESULT_LIMIT: usize = 10;

#[derive(Clone, Debug, PartialEq)]
pub struct SearchCandidate {
    pub fragment: Fragment,
    pub lexical_score: f64,
    pub semantic_score: f64,
    pub combined_score: f64,
    pub generation_id: Option<VectorGenerationId>,
}
