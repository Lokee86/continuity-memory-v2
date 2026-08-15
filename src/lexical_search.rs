use crate::{Cva, DEFAULT_SEARCH_CANDIDATE_LIMIT, Fragment, SearchError};
use std::collections::{HashMap, HashSet};

pub(crate) struct LexicalHit {
    pub fragment: Fragment,
    pub score: f64,
}

impl Cva {
    pub(crate) fn lexical_candidates(
        &mut self,
        query: &str,
    ) -> Result<Vec<LexicalHit>, SearchError> {
        let terms = lexical_terms(query);
        if terms.is_empty() {
            return Ok(Vec::new());
        }
        let mut hits = Vec::new();
        for fragment in self.fragments() {
            let text = self.fragment_text(fragment.id)?;
            let score = lexical_score(&text, &terms);
            if score > 0.0 {
                let archive_version = self
                    .archive
                    .fragments
                    .archive_version(fragment.id)
                    .unwrap_or(0);
                hits.push((LexicalHit { fragment, score }, archive_version));
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
        hits.truncate(DEFAULT_SEARCH_CANDIDATE_LIMIT);
        Ok(hits.into_iter().map(|(hit, _)| hit).collect())
    }
}

pub(crate) fn lexical_score(text: &str, terms: &[String]) -> f64 {
    let words = term_counts(text);
    let mut matched_terms = 0_usize;
    let mut frequency = 0_usize;
    for term in terms {
        if let Some(count) = words.get(term).copied().filter(|count| *count > 0) {
            matched_terms += 1;
            frequency += count.min(4);
        }
    }
    if matched_terms == 0 {
        return 0.0;
    }
    let coverage = matched_terms as f64 / terms.len() as f64;
    let density = (frequency as f64 / (terms.len() * 2) as f64).min(1.0);
    0.85 * coverage + 0.15 * density
}

pub(crate) fn lexical_terms(value: &str) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut terms = Vec::new();
    let mut word = String::new();
    let flush = |word: &mut String, seen: &mut HashSet<String>, terms: &mut Vec<String>| {
        if word.is_empty() {
            return;
        }
        let term = word.to_lowercase();
        word.clear();
        if seen.insert(term.clone()) {
            terms.push(term);
        }
    };
    for current in value.chars() {
        if current.is_alphanumeric() {
            word.push(current);
        } else {
            flush(&mut word, &mut seen, &mut terms);
        }
    }
    flush(&mut word, &mut seen, &mut terms);
    terms
}

fn term_counts(value: &str) -> HashMap<String, usize> {
    let lower = value.to_lowercase();
    lexical_terms(value)
        .into_iter()
        .map(|term| {
            let count = lower.matches(&term).count();
            (term, count)
        })
        .collect()
}
