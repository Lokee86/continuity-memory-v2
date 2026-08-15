use crate::{ArchiveError, SemanticSearchError};
use std::fmt;

#[derive(Debug)]
pub enum SearchError {
    Archive(ArchiveError),
    Semantic(SemanticSearchError),
    EmptyQuery,
}

impl fmt::Display for SearchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "search error: {self:?}")
    }
}

impl std::error::Error for SearchError {}

impl From<ArchiveError> for SearchError {
    fn from(value: ArchiveError) -> Self {
        Self::Archive(value)
    }
}

impl From<SemanticSearchError> for SearchError {
    fn from(value: SemanticSearchError) -> Self {
        Self::Semantic(value)
    }
}
