use crate::community_routing::{RepresentativeStrategy, cosine, routing_profiles};
use crate::community_routing_bench_fixture::{RoutingFixture, RoutingProbe, load_fixture};
use crate::{CommunityId, MemoryId};
use std::collections::{HashMap, HashSet};
use std::time::Instant;

const KS: [usize; 2] = [3, 5];

struct RoutingEntry {
    community_id: CommunityId,
    vector: Vec<f32>,
}

#[test]
fn community_routing_holdout_benchmark() {
    let fixture = load_fixture();
    let held_out: HashSet<_> = fixture
        .vectors
        .keys()
        .copied()
        .filter(|id| id.0[0] % 5 == 0)
        .collect();
    let training: HashMap<_, _> = fixture
        .vectors
        .iter()
        .filter(|(id, _)| !held_out.contains(id))
        .map(|(id, vector)| (*id, vector.clone()))
        .collect();
    let probes: Vec<_> = fixture
        .semantic
        .iter()
        .filter(|probe| held_out.contains(&probe.query))
        .cloned()
        .collect();
    println!(
        "queries={} training_vectors={} held_out={} communities={}",
        probes.len(),
        training.len(),
        held_out.len(),
        fixture.snapshot.communities.len()
    );
    println!("strategy,reps,build_ms,r3,r5,admit3,admit5,avoid5,route_us");

    benchmark(&fixture, &probes, "centroid", || {
        centroids(&fixture, &training)
    });
    for count in [2_usize, 4, 8] {
        benchmark(&fixture, &probes, &format!("subcentroid-{count}"), || {
            subcentroid_entries(&fixture, &training, count)
        });
    }
    benchmark(&fixture, &probes, "structural-8", || {
        representative_entries(
            &fixture,
            &training,
            RepresentativeStrategy::StructuralCentral,
            8,
        )
    });
    benchmark(&fixture, &probes, "diverse-8", || {
        representative_entries(&fixture, &training, RepresentativeStrategy::Diverse, 8)
    });
}

fn benchmark(
    fixture: &RoutingFixture,
    probes: &[RoutingProbe],
    name: &str,
    build: impl FnOnce() -> Vec<RoutingEntry>,
) {
    let started = Instant::now();
    let entries = build();
    let build_ms = started.elapsed().as_secs_f64() * 1000.0;
    let mut hits = [0_usize; KS.len()];
    let mut admitted = [0_usize; KS.len()];
    let mut targets = 0_usize;
    let mut slots = 0_usize;
    let mut route_ns = 0_u128;
    for probe in probes {
        let query = fixture.vectors.get(&probe.query).unwrap();
        let started = Instant::now();
        let ranked = route(&entries, query, *KS.last().unwrap());
        route_ns += started.elapsed().as_nanos();
        targets += probe.targets.len();
        slots += fixture.vectors.len().saturating_sub(1);
        for (slot, k) in KS.into_iter().enumerate() {
            let selected: HashSet<_> = ranked.iter().copied().take(k).collect();
            hits[slot] += probe
                .targets
                .iter()
                .filter(|target| target_admitted(fixture, **target, &selected))
                .count();
            admitted[slot] += admitted_count(fixture, probe.query, &selected);
        }
    }
    let recall = hits.map(|value| value as f64 / targets.max(1) as f64);
    let admit = admitted.map(|value| value as f64 / slots.max(1) as f64);
    let route_us = route_ns as f64 / probes.len().max(1) as f64 / 1000.0;
    println!(
        "{name},{},{build_ms:.3},{:.4},{:.4},{:.4},{:.4},{:.4},{route_us:.3}",
        entries.len(),
        recall[0],
        recall[1],
        admit[0],
        admit[1],
        1.0 - admit[1]
    );
}

fn centroids(fixture: &RoutingFixture, vectors: &HashMap<MemoryId, Vec<f32>>) -> Vec<RoutingEntry> {
    fixture
        .snapshot
        .communities
        .iter()
        .filter_map(|community| {
            mean_vector(
                community
                    .members
                    .iter()
                    .filter_map(|member| vectors.get(member)),
            )
            .map(|vector| RoutingEntry {
                community_id: community.id,
                vector,
            })
        })
        .collect()
}

fn subcentroid_entries(
    fixture: &RoutingFixture,
    vectors: &HashMap<MemoryId, Vec<f32>>,
    count: usize,
) -> Vec<RoutingEntry> {
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
                .map(|vector| RoutingEntry {
                    community_id: community.id,
                    vector,
                })
                .collect::<Vec<_>>()
        })
        .collect()
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

fn representative_entries(
    fixture: &RoutingFixture,
    vectors: &HashMap<MemoryId, Vec<f32>>,
    strategy: RepresentativeStrategy,
    count: usize,
) -> Vec<RoutingEntry> {
    routing_profiles(
        &fixture.snapshot,
        "holdout-routing-benchmark",
        vectors,
        &fixture.relations,
        strategy,
        count,
    )
    .into_iter()
    .flat_map(|profile| {
        profile.representatives.into_iter().filter_map(move |item| {
            vectors.get(&item.memory_id).map(|vector| RoutingEntry {
                community_id: profile.community_id,
                vector: vector.clone(),
            })
        })
    })
    .collect()
}

fn route(entries: &[RoutingEntry], query: &[f32], limit: usize) -> Vec<CommunityId> {
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

fn admitted_count(
    fixture: &RoutingFixture,
    query: MemoryId,
    selected: &HashSet<CommunityId>,
) -> usize {
    fixture
        .vectors
        .keys()
        .filter(|memory| **memory != query)
        .filter(|memory| target_admitted(fixture, **memory, selected))
        .count()
}

fn target_admitted(
    fixture: &RoutingFixture,
    target: MemoryId,
    selected: &HashSet<CommunityId>,
) -> bool {
    match fixture.memberships.get(&target) {
        Some(community) => selected.contains(community),
        None => true,
    }
}
