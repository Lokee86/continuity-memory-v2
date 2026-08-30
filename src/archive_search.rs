use crate::{Cva, Fragment, SearchError};

pub const MAX_ARCHIVE_SEARCH_QUERY_BYTES: usize = 512;
pub const MAX_ARCHIVE_SEARCH_RESULTS: usize = 10;

#[derive(Clone, Debug, PartialEq)]
pub struct ArchiveSearchHit {
    pub fragment: Fragment,
    pub text: String,
    pub score: f64,
}

impl Cva {
    pub fn search_archive(
        &mut self,
        query: &str,
        limit: usize,
    ) -> Result<Vec<ArchiveSearchHit>, SearchError> {
        let query = query.trim();
        if query.is_empty() || query.len() > MAX_ARCHIVE_SEARCH_QUERY_BYTES {
            return Err(SearchError::EmptyQuery);
        }
        if !(1..=MAX_ARCHIVE_SEARCH_RESULTS).contains(&limit) {
            return Err(SearchError::InvalidConfig);
        }
        self.lexical_candidates(query, limit)?
            .into_iter()
            .map(|hit| {
                let text = self.fragment_text(hit.fragment.id)?;
                Ok(ArchiveSearchHit {
                    fragment: hit.fragment,
                    text,
                    score: hit.score,
                })
            })
            .collect()
    }
}
