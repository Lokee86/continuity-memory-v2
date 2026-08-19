use crate::lexical_search::{LexicalHit, term_counts};
use crate::{Archive, ArchiveError, Container, Fragment};
use std::collections::HashMap;

#[derive(Default)]
pub(crate) struct LexicalIndex {
    fragments: Vec<IndexedFragment>,
    postings: HashMap<String, Vec<Posting>>,
}

#[derive(Clone)]
struct IndexedFragment {
    fragment: Fragment,
    archive_version: u64,
}

#[derive(Clone, Copy)]
struct Posting {
    fragment_slot: usize,
    frequency: u8,
}

impl LexicalIndex {
    pub(crate) fn ensure_current(
        &mut self,
        archive: &Archive,
        container: &mut Container,
    ) -> Result<(), ArchiveError> {
        for fragment in archive.fragments.iter().skip(self.fragments.len()) {
            let text = archive.fragment_text(container, fragment.id)?;
            let counts = term_counts(&text);
            let slot = self.fragments.len();
            for (term, count) in counts {
                self.postings.entry(term).or_default().push(Posting {
                    fragment_slot: slot,
                    frequency: u8::try_from(count.min(4)).unwrap_or(4),
                });
            }
            self.fragments.push(IndexedFragment {
                fragment: fragment.clone(),
                archive_version: archive.fragments.archive_version(fragment.id).unwrap_or(0),
            });
        }
        Ok(())
    }

    pub(crate) fn search(&self, terms: &[String], limit: usize) -> Vec<LexicalHit> {
        if terms.is_empty() || limit == 0 {
            return Vec::new();
        }
        let mut scores: HashMap<usize, ScoreParts> = HashMap::new();
        for term in terms {
            let Some(postings) = self.postings.get(term) else {
                continue;
            };
            for posting in postings {
                let entry = scores.entry(posting.fragment_slot).or_default();
                entry.matched_terms += 1;
                entry.frequency += usize::from(posting.frequency);
            }
        }
        let mut hits = scores
            .into_iter()
            .map(|(slot, parts)| {
                let indexed = &self.fragments[slot];
                let coverage = parts.matched_terms as f64 / terms.len() as f64;
                let density = (parts.frequency as f64 / (terms.len() * 2) as f64).min(1.0);
                (
                    LexicalHit {
                        fragment: indexed.fragment.clone(),
                        score: 0.85 * coverage + 0.15 * density,
                    },
                    indexed.archive_version,
                )
            })
            .collect::<Vec<_>>();
        hits.sort_by(|left, right| {
            right
                .0
                .score
                .total_cmp(&left.0.score)
                .then_with(|| right.1.cmp(&left.1))
                .then_with(|| left.0.fragment.id.0.cmp(&right.0.fragment.id.0))
        });
        hits.truncate(limit);
        hits.into_iter().map(|(hit, _)| hit).collect()
    }
}

#[derive(Default)]
struct ScoreParts {
    matched_terms: usize,
    frequency: usize,
}
