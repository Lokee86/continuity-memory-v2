use crate::community_routing::cosine;
use crate::community_routing_bench_fixture::{RoutingFixture, load_fixture};
use crate::community_routing_bench_support::{BenchStrategy, entries, route};
use crate::{CommunityId, MemoryId};
use std::collections::{HashMap, HashSet};

const ROUTE_KS: [usize; 8] = [1, 2, 3, 4, 5, 6, 7, 8];
const ESCAPE_COUNTS: [usize; 4] = [0, 1, 2, 4];

#[derive(Default)]
struct Metrics {
    hits: usize,
    targets: usize,
    admitted: usize,
    candidate_slots: usize,
    escape_unique: usize,
}

#[test]
fn community_retrieval_escape_benchmark() {
    let fixture = load_fixture();
    let mut metrics = HashMap::<(usize, usize), Metrics>::new();
    for probe in &fixture.semantic {
        let query = fixture.vectors.get(&probe.query).expect("query vector");
        let routing_entries = entries(&fixture, BenchStrategy::Centroid, Some(probe.query));
        let ranked_communities = route(&routing_entries, query, *ROUTE_KS.last().unwrap());
        let ranked_targets = ranked_targets(&fixture, probe.query, &probe.targets);
        for k in ROUTE_KS {
            let selected: HashSet<_> = ranked_communities.iter().copied().take(k).collect();
            let admitted = admitted_count(&fixture, probe.query, &selected);
            for escape in ESCAPE_COUNTS {
                let escape_targets: HashSet<_> =
                    ranked_targets.iter().copied().take(escape).collect();
                let row = metrics.entry((k, escape)).or_default();
                row.targets += probe.targets.len();
                row.admitted += admitted;
                row.candidate_slots += fixture.vectors.len().saturating_sub(1);
                for target in &probe.targets {
                    let routed = target_admitted(&fixture, *target, &selected);
                    let escaped = escape_targets.contains(target);
                    if routed || escaped {
                        row.hits += 1;
                    }
                    if escaped && !routed {
                        row.escape_unique += 1;
                    }
                }
            }
        }
    }

    println!("strategy,k,escape,recall,community_admit,community_avoid,escape_unique_per_query");
    for k in ROUTE_KS {
        for escape in ESCAPE_COUNTS {
            let row = metrics.get(&(k, escape)).unwrap();
            let recall = row.hits as f64 / row.targets.max(1) as f64;
            let admitted = row.admitted as f64 / row.candidate_slots.max(1) as f64;
            let escaped = row.escape_unique as f64 / fixture.semantic.len().max(1) as f64;
            println!(
                "centroid,{k},{escape},{recall:.4},{admitted:.4},{:.4},{escaped:.3}",
                1.0 - admitted
            );
        }
    }
}

#[test]
fn community_escape_hint_benchmark() {
    let fixture = load_fixture();
    println!("strategy,k,escape_hints,recall,admit,avoid,communities_added_per_query");
    for k in ROUTE_KS {
        for escape in [1_usize, 2, 4] {
            let mut hits = 0_usize;
            let mut targets = 0_usize;
            let mut admitted = 0_usize;
            let mut slots = 0_usize;
            let mut added = 0_usize;
            for probe in &fixture.semantic {
                let query = fixture.vectors.get(&probe.query).unwrap();
                let routing_entries = entries(&fixture, BenchStrategy::Centroid, Some(probe.query));
                let ranked_communities = route(&routing_entries, query, *ROUTE_KS.last().unwrap());
                let mut selected: HashSet<_> = ranked_communities.iter().copied().take(k).collect();
                let before = selected.len();
                for target in ranked_targets(&fixture, probe.query, &probe.targets)
                    .into_iter()
                    .take(escape)
                {
                    if let Some(community) = fixture.memberships.get(&target) {
                        selected.insert(*community);
                    }
                }
                added += selected.len().saturating_sub(before);
                hits += probe
                    .targets
                    .iter()
                    .filter(|target| target_admitted(&fixture, **target, &selected))
                    .count();
                targets += probe.targets.len();
                admitted += admitted_count(&fixture, probe.query, &selected);
                slots += fixture.vectors.len().saturating_sub(1);
            }
            let recall = hits as f64 / targets.max(1) as f64;
            let admit = admitted as f64 / slots.max(1) as f64;
            let added = added as f64 / fixture.semantic.len().max(1) as f64;
            println!(
                "centroid,{k},{escape},{recall:.4},{admit:.4},{:.4},{added:.3}",
                1.0 - admit
            );
        }
    }
}

fn ranked_targets(
    fixture: &RoutingFixture,
    query: MemoryId,
    targets: &[MemoryId],
) -> Vec<MemoryId> {
    let query_vector = fixture.vectors.get(&query).unwrap();
    let mut ranked: Vec<_> = targets
        .iter()
        .filter_map(|target| {
            cosine(query_vector, fixture.vectors.get(target)?).map(|score| (*target, score))
        })
        .collect();
    ranked.sort_by(|left, right| {
        right
            .1
            .total_cmp(&left.1)
            .then_with(|| left.0.0.cmp(&right.0.0))
    });
    ranked.into_iter().map(|(id, _)| id).collect()
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
