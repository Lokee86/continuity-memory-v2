use crate::community_routing_bench_fixture::{RoutingFixture, load_fixture};
use crate::community_routing_bench_support::{BenchStrategy, entries, route};
use crate::dream_candidate_ranking::lexical_score;
use crate::{CommunityId, MemoryId};
use std::collections::{HashMap, HashSet};

const KS: [usize; 3] = [3, 4, 5];
const HINTS: [usize; 3] = [1, 2, 4];

#[derive(Default)]
struct Metrics {
    hits: usize,
    targets: usize,
    admitted: usize,
    slots: usize,
    added: usize,
}

#[test]
#[ignore = "lexical global escape hits used as community-routing hints"]
fn community_lexical_escape_hint_benchmark() {
    let fixture = load_fixture();
    let mut metrics = HashMap::<(usize, usize), Metrics>::new();
    for probe in &fixture.semantic {
        let query_vector = fixture.vectors.get(&probe.query).unwrap();
        let routing_entries = entries(&fixture, BenchStrategy::Centroid, Some(probe.query));
        let ranked_communities = route(&routing_entries, query_vector, *KS.last().unwrap());
        let lexical = lexical_hits(&fixture, probe.query, *HINTS.last().unwrap());
        for k in KS {
            for hints in HINTS {
                let mut selected: HashSet<_> = ranked_communities.iter().copied().take(k).collect();
                let before = selected.len();
                for target in lexical.iter().copied().take(hints) {
                    if let Some(community) = fixture.memberships.get(&target) {
                        selected.insert(*community);
                    }
                }
                let row = metrics.entry((k, hints)).or_default();
                row.added += selected.len().saturating_sub(before);
                row.hits += probe
                    .targets
                    .iter()
                    .filter(|target| target_admitted(&fixture, **target, &selected))
                    .count();
                row.targets += probe.targets.len();
                row.admitted += admitted_count(&fixture, probe.query, &selected);
                row.slots += fixture.vectors.len().saturating_sub(1);
            }
        }
    }

    println!("strategy,k,lexical_hints,recall,admit,avoid,communities_added_per_query");
    for k in KS {
        for hints in HINTS {
            let row = metrics.get(&(k, hints)).unwrap();
            let recall = row.hits as f64 / row.targets.max(1) as f64;
            let admit = row.admitted as f64 / row.slots.max(1) as f64;
            let added = row.added as f64 / fixture.semantic.len().max(1) as f64;
            println!(
                "centroid+lexical,{k},{hints},{recall:.4},{admit:.4},{:.4},{added:.3}",
                1.0 - admit
            );
        }
    }
}

fn lexical_hits(fixture: &RoutingFixture, query: MemoryId, limit: usize) -> Vec<MemoryId> {
    let source = fixture.memories.get(&query).unwrap();
    let mut ranked: Vec<_> = fixture
        .memories
        .iter()
        .filter(|(id, _)| **id != query)
        .map(|(id, memory)| (*id, lexical_score(source, memory)))
        .filter(|(_, score)| *score > 0.0)
        .collect();
    ranked.sort_by(|left, right| {
        right
            .1
            .total_cmp(&left.1)
            .then_with(|| left.0.0.cmp(&right.0.0))
    });
    ranked.into_iter().take(limit).map(|(id, _)| id).collect()
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
