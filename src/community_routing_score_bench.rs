use crate::community_routing::{RepresentativeStrategy, cosine, routing_profiles};
use crate::community_routing_bench_fixture::{RoutingFixture, load_fixture};
use crate::{CommunityId, MemoryId};
use std::collections::{HashMap, HashSet};

const K: usize = 5;

struct RepEntry {
    community_id: CommunityId,
    memory_id: MemoryId,
    vector: Vec<f32>,
}

struct CentroidStats {
    count: usize,
    sum: Vec<f64>,
}

#[test]
#[ignore = "community routing score-composition experiment"]
fn community_routing_score_composition_benchmark() {
    let fixture = load_fixture();
    let centroids = centroid_stats(&fixture);
    println!("reps,score,recall,admit,avoid");
    evaluate(&fixture, &centroids, "none", &[], "centroid", 0.0, false);
    for (name, strategy) in [
        ("structural-8", RepresentativeStrategy::StructuralCentral),
        ("diverse-8", RepresentativeStrategy::Diverse),
    ] {
        let entries = rep_entries(&fixture, strategy, 8);
        evaluate(&fixture, &centroids, name, &entries, "max", 0.0, false);
        evaluate(&fixture, &centroids, name, &entries, "mean2", 0.0, true);
        for alpha in [0.25, 0.5, 1.0, 2.0] {
            evaluate(
                &fixture,
                &centroids,
                name,
                &entries,
                "blend-max",
                alpha,
                false,
            );
            evaluate(
                &fixture,
                &centroids,
                name,
                &entries,
                "blend-mean2",
                alpha,
                true,
            );
        }
    }
}

fn evaluate(
    fixture: &RoutingFixture,
    centroids: &HashMap<CommunityId, CentroidStats>,
    reps_name: &str,
    entries: &[RepEntry],
    score_name: &str,
    alpha: f64,
    mean_two: bool,
) {
    let mut hits = 0_usize;
    let mut targets = 0_usize;
    let mut admitted = 0_usize;
    let mut slots = 0_usize;
    for probe in &fixture.semantic {
        let query = fixture.vectors.get(&probe.query).unwrap();
        let centroid_scores = centroid_scores(fixture, centroids, probe.query, query);
        let rep_scores = representative_scores(entries, probe.query, query, mean_two);
        let mut ranked: Vec<_> = fixture
            .snapshot
            .communities
            .iter()
            .map(|community| {
                let centroid = centroid_scores
                    .get(&community.id)
                    .copied()
                    .unwrap_or(f64::NEG_INFINITY);
                let rep = rep_scores
                    .get(&community.id)
                    .copied()
                    .unwrap_or(f64::NEG_INFINITY);
                let score = match score_name {
                    "centroid" => centroid,
                    "max" | "mean2" => rep,
                    _ if rep.is_finite() => centroid + alpha * rep,
                    _ => centroid,
                };
                (community.id, score)
            })
            .collect();
        ranked.sort_by(|left, right| {
            right
                .1
                .total_cmp(&left.1)
                .then_with(|| left.0.cmp(&right.0))
        });
        let selected: HashSet<_> = ranked.iter().take(K).map(|(id, _)| *id).collect();
        hits += probe
            .targets
            .iter()
            .filter(|target| target_admitted(fixture, **target, &selected))
            .count();
        targets += probe.targets.len();
        admitted += admitted_count(fixture, probe.query, &selected);
        slots += fixture.vectors.len().saturating_sub(1);
    }
    let recall = hits as f64 / targets.max(1) as f64;
    let admit = admitted as f64 / slots.max(1) as f64;
    let label = if score_name.starts_with("blend") {
        format!("{score_name}-{alpha:.2}")
    } else {
        score_name.to_owned()
    };
    println!(
        "{reps_name},{label},{recall:.4},{admit:.4},{:.4}",
        1.0 - admit
    );
}

fn centroid_stats(fixture: &RoutingFixture) -> HashMap<CommunityId, CentroidStats> {
    fixture
        .snapshot
        .communities
        .iter()
        .filter_map(|community| {
            let vectors: Vec<_> = community
                .members
                .iter()
                .filter_map(|member| fixture.vectors.get(member))
                .collect();
            let dimensions = vectors.first()?.len();
            let mut sum = vec![0.0_f64; dimensions];
            for vector in &vectors {
                for (target, value) in sum.iter_mut().zip(vector.iter()) {
                    *target += *value as f64;
                }
            }
            Some((
                community.id,
                CentroidStats {
                    count: vectors.len(),
                    sum,
                },
            ))
        })
        .collect()
}

fn centroid_scores(
    fixture: &RoutingFixture,
    centroids: &HashMap<CommunityId, CentroidStats>,
    query_id: MemoryId,
    query: &[f32],
) -> HashMap<CommunityId, f64> {
    centroids
        .iter()
        .filter_map(|(community, stats)| {
            let exclude = fixture.memberships.get(&query_id) == Some(community);
            let count = stats.count.saturating_sub(usize::from(exclude));
            if count == 0 {
                return None;
            }
            let vector: Vec<f32> = stats
                .sum
                .iter()
                .enumerate()
                .map(|(index, value)| {
                    let adjusted = value - if exclude { query[index] as f64 } else { 0.0 };
                    (adjusted / count as f64) as f32
                })
                .collect();
            cosine(query, &vector).map(|score| (*community, score))
        })
        .collect()
}

fn rep_entries(
    fixture: &RoutingFixture,
    strategy: RepresentativeStrategy,
    count: usize,
) -> Vec<RepEntry> {
    routing_profiles(
        &fixture.snapshot,
        "score-benchmark",
        &fixture.vectors,
        &fixture.relations,
        strategy,
        count,
    )
    .into_iter()
    .flat_map(|profile| {
        profile.representatives.into_iter().filter_map(move |item| {
            fixture.vectors.get(&item.memory_id).map(|vector| RepEntry {
                community_id: profile.community_id,
                memory_id: item.memory_id,
                vector: vector.clone(),
            })
        })
    })
    .collect()
}

fn representative_scores(
    entries: &[RepEntry],
    query_id: MemoryId,
    query: &[f32],
    mean_two: bool,
) -> HashMap<CommunityId, f64> {
    let mut grouped = HashMap::<CommunityId, Vec<f64>>::new();
    for entry in entries.iter().filter(|entry| entry.memory_id != query_id) {
        if let Some(score) = cosine(query, &entry.vector) {
            grouped.entry(entry.community_id).or_default().push(score);
        }
    }
    grouped
        .into_iter()
        .map(|(community, mut scores)| {
            scores.sort_by(|left, right| right.total_cmp(left));
            let score = if mean_two {
                scores.iter().take(2).sum::<f64>() / scores.len().min(2).max(1) as f64
            } else {
                scores[0]
            };
            (community, score)
        })
        .collect()
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
