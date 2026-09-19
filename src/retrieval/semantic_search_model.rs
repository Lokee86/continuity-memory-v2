use crate::{Fragment, VectorGenerationId};

pub const MAX_SEMANTIC_SEARCH_LIMIT: usize = 1000;

#[derive(Clone, Debug, PartialEq)]
pub struct SemanticSearchHit {
    pub fragment: Fragment,
    pub score: f64,
    pub ordinal: u64,
    pub generation_id: VectorGenerationId,
}
