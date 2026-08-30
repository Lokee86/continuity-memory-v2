use crate::fragmenter::path_fragment_windows;
use crate::lexical_search::{lexical_score, lexical_terms};
use crate::{ConversationSearchHit, Cva, FragmentConfig, SearchError};

impl Cva {
    pub fn search_conversation_branch(
        &mut self,
        conversation_id: &str,
        leaf_node_id: &str,
        query: &str,
        limit: usize,
    ) -> Result<Vec<ConversationSearchHit>, SearchError> {
        if query.trim().is_empty() {
            return Err(SearchError::EmptyQuery);
        }
        if !(1..=crate::MAX_SEMANTIC_SEARCH_LIMIT).contains(&limit) {
            return Err(SearchError::InvalidConfig);
        }
        let terms = lexical_terms(query);
        if terms.is_empty() {
            return Ok(Vec::new());
        }
        let nodes = self.archive.branch_nodes(conversation_id, leaf_node_id)?;
        let fragments = path_fragment_windows(&nodes, FragmentConfig::default(), true)?;
        let mut hits = Vec::new();
        for (position, fragment) in fragments.into_iter().enumerate() {
            let text = self
                .archive
                .fragment_text_for(&mut self.container, &fragment)?;
            let score = lexical_score(&text, &terms);
            if score > 0.0 {
                hits.push((
                    ConversationSearchHit {
                        fragment,
                        text,
                        score,
                    },
                    position,
                ));
            }
        }
        hits.sort_by(|left, right| {
            right
                .0
                .score
                .total_cmp(&left.0.score)
                .then_with(|| right.1.cmp(&left.1))
                .then_with(|| left.0.fragment.id.0.cmp(&right.0.fragment.id.0))
        });
        hits.truncate(limit);
        Ok(hits.into_iter().map(|(hit, _)| hit).collect())
    }
}
