use crate::{CommunityId, CommunitySnapshot, GraphRelation, MemoryId, MemoryRef};
use std::collections::{HashMap, HashSet};

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct CommunityRoutingProfile {
    pub(crate) community_id: CommunityId,
    pub(crate) representatives: Vec<MemoryRef>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum RepresentativeStrategy {
    SingleMedoid,
    Diverse,
    StructuralCentral,
}

pub(crate) fn routing_profiles(
    snapshot: &CommunitySnapshot,
    owner_id: &str,
    vectors: &HashMap<MemoryId, Vec<f32>>,
    relations: &[GraphRelation],
    strategy: RepresentativeStrategy,
    limit: usize,
) -> Vec<CommunityRoutingProfile> {
    snapshot
        .communities
        .iter()
        .map(|community| {
            let ids = match strategy {
                RepresentativeStrategy::SingleMedoid => {
                    medoid(&community.members, vectors).into_iter().collect()
                }
                RepresentativeStrategy::Diverse => diverse(&community.members, vectors, limit),
                RepresentativeStrategy::StructuralCentral => {
                    structural_central(&community.members, vectors, relations, limit)
                }
            };
            CommunityRoutingProfile {
                community_id: community.id,
                representatives: ids
                    .into_iter()
                    .map(|memory_id| MemoryRef {
                        owner_id: owner_id.to_owned(),
                        memory_id,
                    })
                    .collect(),
            }
        })
        .collect()
}

pub(crate) fn cosine(left: &[f32], right: &[f32]) -> Option<f64> {
    if left.len() != right.len() || left.is_empty() {
        return None;
    }
    let mut dot = 0.0_f64;
    let mut left_norm = 0.0_f64;
    let mut right_norm = 0.0_f64;
    for (left, right) in left.iter().zip(right) {
        dot += *left as f64 * *right as f64;
        left_norm += (*left as f64).powi(2);
        right_norm += (*right as f64).powi(2);
    }
    (left_norm > 0.0 && right_norm > 0.0 && dot.is_finite())
        .then(|| dot / (left_norm.sqrt() * right_norm.sqrt()))
}

fn medoid(members: &[MemoryId], vectors: &HashMap<MemoryId, Vec<f32>>) -> Option<MemoryId> {
    let candidates = vectorized_members(members, vectors);
    candidates.iter().copied().max_by(|left, right| {
        medoid_score(*left, &candidates, vectors)
            .total_cmp(&medoid_score(*right, &candidates, vectors))
            .then_with(|| right.0.cmp(&left.0))
    })
}

fn medoid_score(
    candidate: MemoryId,
    members: &[MemoryId],
    vectors: &HashMap<MemoryId, Vec<f32>>,
) -> f64 {
    let Some(candidate_vector) = vectors.get(&candidate) else {
        return f64::NEG_INFINITY;
    };
    members
        .iter()
        .filter_map(|member| cosine(candidate_vector, vectors.get(member)?))
        .sum()
}

fn diverse(
    members: &[MemoryId],
    vectors: &HashMap<MemoryId, Vec<f32>>,
    limit: usize,
) -> Vec<MemoryId> {
    if limit == 0 {
        return Vec::new();
    }
    let candidates = vectorized_members(members, vectors);
    let Some(first) = medoid(&candidates, vectors) else {
        return Vec::new();
    };
    let mut selected = vec![first];
    while selected.len() < limit.min(candidates.len()) {
        let next = candidates
            .iter()
            .copied()
            .filter(|candidate| !selected.contains(candidate))
            .min_by(|left, right| {
                nearest_similarity(*left, &selected, vectors)
                    .total_cmp(&nearest_similarity(*right, &selected, vectors))
                    .then_with(|| left.0.cmp(&right.0))
            });
        let Some(next) = next else { break };
        selected.push(next);
    }
    selected
}

fn nearest_similarity(
    candidate: MemoryId,
    selected: &[MemoryId],
    vectors: &HashMap<MemoryId, Vec<f32>>,
) -> f64 {
    let Some(candidate_vector) = vectors.get(&candidate) else {
        return f64::INFINITY;
    };
    selected
        .iter()
        .filter_map(|id| cosine(candidate_vector, vectors.get(id)?))
        .fold(f64::NEG_INFINITY, f64::max)
}

fn structural_central(
    members: &[MemoryId],
    vectors: &HashMap<MemoryId, Vec<f32>>,
    relations: &[GraphRelation],
    limit: usize,
) -> Vec<MemoryId> {
    let member_set: HashSet<_> = members.iter().copied().collect();
    let mut seen = HashSet::new();
    let mut degree = HashMap::<MemoryId, usize>::new();
    for relation in relations.iter().filter(|relation| relation.active) {
        if !member_set.contains(&relation.source) || !member_set.contains(&relation.target) {
            continue;
        }
        let pair = ordered(relation.source, relation.target);
        if seen.insert(pair) {
            *degree.entry(pair.0).or_default() += 1;
            *degree.entry(pair.1).or_default() += 1;
        }
    }
    let mut candidates = vectorized_members(members, vectors);
    candidates.sort_by(|left, right| {
        degree
            .get(right)
            .copied()
            .unwrap_or(0)
            .cmp(&degree.get(left).copied().unwrap_or(0))
            .then_with(|| left.0.cmp(&right.0))
    });
    candidates.truncate(limit.min(candidates.len()));
    candidates
}

fn vectorized_members(
    members: &[MemoryId],
    vectors: &HashMap<MemoryId, Vec<f32>>,
) -> Vec<MemoryId> {
    let mut output: Vec<_> = members
        .iter()
        .copied()
        .filter(|member| vectors.contains_key(member))
        .collect();
    output.sort_by_key(|member| member.0);
    output
}

fn ordered(left: MemoryId, right: MemoryId) -> (MemoryId, MemoryId) {
    if left.0 <= right.0 {
        (left, right)
    } else {
        (right, left)
    }
}
