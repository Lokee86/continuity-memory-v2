use crate::{DreamCandidate, DreamCandidateConfig, DreamMemoryContext, Memory};
use std::collections::HashSet;

#[derive(Clone)]
pub(crate) struct ScoredCandidate {
    pub(crate) context: DreamMemoryContext,
    pub(crate) semantic_score: Option<f64>,
    pub(crate) lexical_score: f64,
    pub(crate) semantic_rank: Option<usize>,
    pub(crate) prior_rank: Option<usize>,
    pub(crate) lexical_rank: Option<usize>,
    pub(crate) temporal_score: f64,
    pub(crate) temporal_rank: Option<usize>,
    pub(crate) temporal_matches: Vec<crate::DreamTemporalMatch>,
    pub(crate) fused_score: f64,
}

pub(crate) fn rank_lanes(
    source: &DreamMemoryContext,
    candidates: &mut [ScoredCandidate],
    config: DreamCandidateConfig,
) {
    let mut semantic: Vec<_> = candidates
        .iter()
        .enumerate()
        .filter_map(|(index, candidate)| {
            candidate
                .semantic_score
                .filter(|score| *score > 0.0)
                .map(|score| (index, score))
        })
        .collect();
    semantic.sort_by(|a, b| {
        b.1.total_cmp(&a.1).then_with(|| {
            candidates[a.0]
                .context
                .memory
                .id
                .0
                .cmp(&candidates[b.0].context.memory.id.0)
        })
    });
    for (rank, (index, _)) in semantic.iter().take(config.semantic_limit).enumerate() {
        candidates[*index].semantic_rank = Some(rank + 1);
    }

    if let Some(source_time) = source.source_timestamp_ns {
        let mut prior: Vec<_> = semantic
            .iter()
            .copied()
            .filter(|(index, _)| {
                candidates[*index]
                    .context
                    .source_timestamp_ns
                    .is_some_and(|timestamp| timestamp < source_time)
            })
            .collect();
        prior.truncate(config.prior_semantic_quota);
        for (rank, (index, _)) in prior.iter().enumerate() {
            candidates[*index].prior_rank = Some(rank + 1);
        }
    }

    let mut lexical: Vec<_> = candidates
        .iter()
        .enumerate()
        .filter(|(_, candidate)| candidate.lexical_score > 0.0)
        .map(|(index, candidate)| (index, candidate.lexical_score))
        .collect();
    lexical.sort_by(|a, b| {
        b.1.total_cmp(&a.1).then_with(|| {
            candidates[a.0]
                .context
                .memory
                .id
                .0
                .cmp(&candidates[b.0].context.memory.id.0)
        })
    });
    for (rank, (index, _)) in lexical.iter().take(config.lexical_limit).enumerate() {
        candidates[*index].lexical_rank = Some(rank + 1);
    }

    let mut temporal: Vec<_> = candidates
        .iter()
        .enumerate()
        .filter(|(_, candidate)| candidate.temporal_score > 0.0)
        .map(|(index, candidate)| (index, candidate.temporal_score))
        .collect();
    temporal.sort_by(|a, b| {
        b.1.total_cmp(&a.1).then_with(|| {
            candidates[a.0]
                .context
                .memory
                .id
                .0
                .cmp(&candidates[b.0].context.memory.id.0)
        })
    });
    for (rank, (index, _)) in temporal.iter().take(config.temporal_limit).enumerate() {
        candidates[*index].temporal_rank = Some(rank + 1);
    }

    for candidate in candidates {
        candidate.fused_score = rrf(candidate.semantic_rank, 1.0)
            + rrf(candidate.prior_rank, 0.6)
            + rrf(candidate.lexical_rank, 0.35)
            + rrf(candidate.temporal_rank, 1.0);
    }
}

pub(crate) fn select_candidates(
    mut candidates: Vec<ScoredCandidate>,
    config: DreamCandidateConfig,
) -> Vec<DreamCandidate> {
    let reserved: HashSet<_> = candidates
        .iter()
        .filter(|candidate| candidate.prior_rank.is_some())
        .map(|candidate| candidate.context.memory.id)
        .collect();
    candidates.retain(|candidate| {
        candidate.semantic_rank.is_some()
            || candidate.prior_rank.is_some()
            || candidate.lexical_rank.is_some()
            || candidate.temporal_rank.is_some()
    });
    candidates.sort_by(|a, b| {
        b.fused_score
            .total_cmp(&a.fused_score)
            .then_with(|| a.context.memory.id.0.cmp(&b.context.memory.id.0))
    });

    let mut selected = Vec::new();
    for candidate in candidates
        .iter()
        .filter(|candidate| reserved.contains(&candidate.context.memory.id))
    {
        if selected.len() == config.limit {
            break;
        }
        selected.push(candidate.clone());
    }
    for candidate in candidates {
        if selected.len() == config.limit {
            break;
        }
        if !selected
            .iter()
            .any(|item| item.context.memory.id == candidate.context.memory.id)
        {
            selected.push(candidate);
        }
    }
    selected.sort_by(|a, b| {
        b.fused_score
            .total_cmp(&a.fused_score)
            .then_with(|| a.context.memory.id.0.cmp(&b.context.memory.id.0))
    });
    selected.into_iter().map(to_public_candidate).collect()
}

pub(crate) fn lexical_score(source: &Memory, candidate: &Memory) -> f64 {
    let source_tokens = tokens(&format!("{} {}", source.title, source.content));
    let candidate_tokens = tokens(&format!("{} {}", candidate.title, candidate.content));
    let shared = source_tokens.intersection(&candidate_tokens).count() as f64;
    let union = source_tokens.union(&candidate_tokens).count() as f64;
    let mut score = if union > 0.0 { shared / union } else { 0.0 };
    if source.category == candidate.category {
        score += 0.08;
    }
    if source.memory_type == candidate.memory_type {
        score += 0.08;
    }
    if source.parent_id.is_some() && source.parent_id == candidate.parent_id {
        score += 0.12;
    }
    score
}

fn to_public_candidate(candidate: ScoredCandidate) -> DreamCandidate {
    DreamCandidate {
        context: candidate.context,
        semantic_score: candidate.semantic_score,
        semantic_rank: candidate.semantic_rank,
        prior_semantic_rank: candidate.prior_rank,
        lexical_score: candidate.lexical_score,
        lexical_rank: candidate.lexical_rank,
        temporal_score: candidate.temporal_score,
        temporal_rank: candidate.temporal_rank,
        temporal_matches: candidate.temporal_matches,
    }
}

fn rrf(rank: Option<usize>, weight: f64) -> f64 {
    rank.map(|rank| weight / (60.0 + rank as f64))
        .unwrap_or(0.0)
}

fn tokens(text: &str) -> HashSet<String> {
    text.split(|ch: char| !ch.is_alphanumeric())
        .filter(|token| token.len() > 1)
        .map(str::to_lowercase)
        .collect()
}
