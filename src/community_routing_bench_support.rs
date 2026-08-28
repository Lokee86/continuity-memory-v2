use crate::community_routing::{RepresentativeStrategy, cosine, routing_profiles};
use crate::community_routing_bench_fixture::RoutingFixture;
use crate::{CommunityId, MemoryId};
use std::collections::HashMap;

#[derive(Clone, Copy, Debug)]
pub(crate) enum BenchStrategy {
    Centroid,
    Medoid,
    Diverse(usize),
    Structural(usize),
}

impl BenchStrategy {
    pub(crate) fn name(self) -> String {
        match self {
            Self::Centroid => "centroid".into(),
            Self::Medoid => "medoid".into(),
            Self::Diverse(count) => format!("diverse-{count}"),
            Self::Structural(count) => format!("structural-{count}"),
        }
    }
}

#[derive(Clone)]
pub(crate) struct RoutingEntry {
    pub(crate) community_id: CommunityId,
    pub(crate) vector: Vec<f32>,
}

pub(crate) fn entries(
    fixture: &RoutingFixture,
    strategy: BenchStrategy,
    excluded: Option<MemoryId>,
) -> Vec<RoutingEntry> {
    let mut vectors = fixture.vectors.clone();
    if let Some(excluded) = excluded {
        vectors.remove(&excluded);
    }
    match strategy {
        BenchStrategy::Centroid => centroids(fixture, &vectors),
        BenchStrategy::Medoid => {
            representatives(fixture, &vectors, RepresentativeStrategy::SingleMedoid, 1)
        }
        BenchStrategy::Diverse(count) => {
            representatives(fixture, &vectors, RepresentativeStrategy::Diverse, count)
        }
        BenchStrategy::Structural(count) => representatives(
            fixture,
            &vectors,
            RepresentativeStrategy::StructuralCentral,
            count,
        ),
    }
}

pub(crate) fn route(entries: &[RoutingEntry], query: &[f32], limit: usize) -> Vec<CommunityId> {
    let mut best = HashMap::<CommunityId, f64>::new();
    for entry in entries {
        let Some(score) = cosine(query, &entry.vector) else {
            continue;
        };
        best.entry(entry.community_id)
            .and_modify(|current| *current = current.max(score))
            .or_insert(score);
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

pub(crate) fn logical_index_bytes(fixture: &RoutingFixture, strategy: BenchStrategy) -> usize {
    let canonical = entries(fixture, strategy, None);
    match strategy {
        BenchStrategy::Centroid => {
            let dimensions = canonical.first().map_or(0, |entry| entry.vector.len());
            canonical.len() * (32 + dimensions * size_of::<f32>())
        }
        _ => canonical.len() * 64,
    }
}

fn centroids(fixture: &RoutingFixture, vectors: &HashMap<MemoryId, Vec<f32>>) -> Vec<RoutingEntry> {
    fixture
        .snapshot
        .communities
        .iter()
        .filter_map(|community| {
            let member_vectors: Vec<_> = community
                .members
                .iter()
                .filter_map(|member| vectors.get(member))
                .collect();
            let dimensions = member_vectors.first()?.len();
            let mut centroid = vec![0.0_f32; dimensions];
            for vector in &member_vectors {
                for (target, value) in centroid.iter_mut().zip(vector.iter()) {
                    *target += *value;
                }
            }
            let scale = 1.0 / member_vectors.len() as f32;
            for value in &mut centroid {
                *value *= scale;
            }
            Some(RoutingEntry {
                community_id: community.id,
                vector: centroid,
            })
        })
        .collect()
}

fn representatives(
    fixture: &RoutingFixture,
    vectors: &HashMap<MemoryId, Vec<f32>>,
    strategy: RepresentativeStrategy,
    count: usize,
) -> Vec<RoutingEntry> {
    routing_profiles(
        &fixture.snapshot,
        "routing-benchmark",
        vectors,
        &fixture.relations,
        strategy,
        count,
    )
    .into_iter()
    .flat_map(|profile| {
        profile
            .representatives
            .into_iter()
            .filter_map(move |reference| {
                vectors
                    .get(&reference.memory_id)
                    .map(|vector| RoutingEntry {
                        community_id: profile.community_id,
                        vector: vector.clone(),
                    })
            })
    })
    .collect()
}
