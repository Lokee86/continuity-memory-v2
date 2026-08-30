use crate::community_routing::cosine;
use crate::community_routing_bench_fixture::RoutingFixture;
use crate::{CommunityId, MemoryId};
use std::collections::HashMap;

pub(crate) struct SubcentroidEntry {
    pub(crate) community_id: CommunityId,
    pub(crate) vector: Vec<f32>,
}

pub(crate) fn build_subcentroids(
    fixture: &RoutingFixture,
    vectors: &HashMap<MemoryId, Vec<f32>>,
    count: usize,
) -> Vec<SubcentroidEntry> {
    fixture
        .snapshot
        .communities
        .iter()
        .flat_map(|community| {
            let points: Vec<_> = community
                .members
                .iter()
                .filter_map(|member| vectors.get(member).cloned())
                .collect();
            subcentroids(&points, count)
                .into_iter()
                .map(|vector| SubcentroidEntry {
                    community_id: community.id,
                    vector,
                })
                .collect::<Vec<_>>()
        })
        .collect()
}

pub(crate) fn route_subcentroids(
    entries: &[SubcentroidEntry],
    query: &[f32],
    limit: usize,
) -> Vec<CommunityId> {
    let mut best = HashMap::<CommunityId, f64>::new();
    for entry in entries {
        if let Some(score) = cosine(query, &entry.vector) {
            best.entry(entry.community_id)
                .and_modify(|current| *current = current.max(score))
                .or_insert(score);
        }
    }
    let mut ranked: Vec<_> = best.into_iter().collect();
    ranked.sort_by(|left, right| {
        right
            .1
            .total_cmp(&left.1)
            .then_with(|| left.0.cmp(&right.0))
    });
    ranked.truncate(limit.min(ranked.len()));
    ranked.into_iter().map(|(community, _)| community).collect()
}

fn subcentroids(points: &[Vec<f32>], count: usize) -> Vec<Vec<f32>> {
    let Some(center) = mean_vector(points.iter()) else {
        return Vec::new();
    };
    let target = count.min(points.len()).max(1);
    let first = points
        .iter()
        .enumerate()
        .max_by(|left, right| {
            cosine(&center, left.1)
                .unwrap_or(f64::NEG_INFINITY)
                .total_cmp(&cosine(&center, right.1).unwrap_or(f64::NEG_INFINITY))
        })
        .map(|(index, _)| index)
        .unwrap();
    let mut seeds = vec![points[first].clone()];
    while seeds.len() < target {
        let next = points
            .iter()
            .min_by(|left, right| {
                nearest_similarity(left, &seeds).total_cmp(&nearest_similarity(right, &seeds))
            })
            .unwrap()
            .clone();
        seeds.push(next);
    }
    for _ in 0..2 {
        let mut groups = vec![Vec::<&Vec<f32>>::new(); seeds.len()];
        for point in points {
            let index = seeds
                .iter()
                .enumerate()
                .max_by(|left, right| {
                    cosine(point, left.1)
                        .unwrap_or(f64::NEG_INFINITY)
                        .total_cmp(&cosine(point, right.1).unwrap_or(f64::NEG_INFINITY))
                })
                .map(|(index, _)| index)
                .unwrap();
            groups[index].push(point);
        }
        for (seed, group) in seeds.iter_mut().zip(groups) {
            if let Some(center) = mean_vector(group.into_iter()) {
                *seed = center;
            }
        }
    }
    seeds
}

fn nearest_similarity(point: &[f32], seeds: &[Vec<f32>]) -> f64 {
    seeds
        .iter()
        .filter_map(|seed| cosine(point, seed))
        .fold(f64::NEG_INFINITY, f64::max)
}

fn mean_vector<'a>(vectors: impl Iterator<Item = &'a Vec<f32>>) -> Option<Vec<f32>> {
    let vectors: Vec<_> = vectors.collect();
    let dimensions = vectors.first()?.len();
    let mut output = vec![0.0_f32; dimensions];
    for vector in &vectors {
        for (target, value) in output.iter_mut().zip(vector.iter()) {
            *target += *value;
        }
    }
    let scale = 1.0 / vectors.len() as f32;
    for value in &mut output {
        *value *= scale;
    }
    Some(output)
}
