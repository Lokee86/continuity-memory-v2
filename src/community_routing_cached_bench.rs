use crate::community_routing::{cosine, routing_profiles, RepresentativeStrategy};
use crate::community_routing_bench_fixture::{load_fixture, RoutingFixture};
use crate::{CommunityId, MemoryId};
use std::collections::{HashMap, HashSet};
use std::time::Instant;

const KS: [usize; 8] = [1, 2, 3, 4, 5, 6, 7, 8];

struct CachedEntry {
    community_id: CommunityId,
    memory_id: MemoryId,
    vector: Vec<f32>,
}

#[test]
#[ignore = "cached representative routing over a larger Dream-populated typed REL"]
fn community_cached_representative_routing_benchmark() {
    let fixture = load_fixture();
    println!("strategy,reps,build_ms,k,recall,admit,avoid,route_us");
    for (name, strategy, count) in [
        ("medoid", RepresentativeStrategy::SingleMedoid, 1),
        ("diverse-4", RepresentativeStrategy::Diverse, 4),
        ("diverse-8", RepresentativeStrategy::Diverse, 8),
        ("structural-4", RepresentativeStrategy::StructuralCentral, 4),
        ("structural-8", RepresentativeStrategy::StructuralCentral, 8),
    ] {
        let started = Instant::now();
        let entries = cached_entries(&fixture, strategy, count);
        let build_ms = started.elapsed().as_secs_f64() * 1000.0;
        let mut hits = [0_usize; KS.len()];
        let mut admitted = [0_usize; KS.len()];
        let mut targets = 0_usize;
        let mut candidate_slots = 0_usize;
        let mut route_ns = 0_u128;
        for probe in &fixture.semantic {
            let query = fixture.vectors.get(&probe.query).unwrap();
            let started = Instant::now();
            let ranked = route_cached(&entries, probe.query, query, *KS.last().unwrap());
            route_ns += started.elapsed().as_nanos();
            targets += probe.targets.len();
            candidate_slots += fixture.vectors.len().saturating_sub(1);
            for (slot, k) in KS.into_iter().enumerate() {
                let selected: HashSet<_> = ranked.iter().copied().take(k).collect();
                hits[slot] += probe
                    .targets
                    .iter()
                    .filter(|target| target_admitted(&fixture, **target, &selected))
                    .count();
                admitted[slot] += admitted_count(&fixture, probe.query, &selected);
            }
        }
        for (slot, k) in KS.into_iter().enumerate() {
            let recall = hits[slot] as f64 / targets.max(1) as f64;
            let admit = admitted[slot] as f64 / candidate_slots.max(1) as f64;
            let route_us = route_ns as f64 / fixture.semantic.len().max(1) as f64 / 1000.0;
            println!(
                "{name},{},{build_ms:.3},{k},{recall:.4},{admit:.4},{:.4},{route_us:.3}",
                entries.len(),
                1.0 - admit
            );
        }
    }
}

fn cached_entries(
    fixture: &RoutingFixture,
    strategy: RepresentativeStrategy,
    count: usize,
) -> Vec<CachedEntry> {
    routing_profiles(
        &fixture.snapshot,
        "cached-routing-benchmark",
        &fixture.vectors,
        &fixture.relations,
        strategy,
        count,
    )
    .into_iter()
    .flat_map(|profile| {
        profile.representatives.into_iter().filter_map(move |item| {
            fixture
                .vectors
                .get(&item.memory_id)
                .map(|vector| CachedEntry {
                    community_id: profile.community_id,
                    memory_id: item.memory_id,
                    vector: vector.clone(),
                })
        })
    })
    .collect()
}

fn route_cached(
    entries: &[CachedEntry],
    excluded: MemoryId,
    query: &[f32],
    limit: usize,
) -> Vec<CommunityId> {
    let mut best = HashMap::<CommunityId, f64>::new();
    for entry in entries.iter().filter(|entry| entry.memory_id != excluded) {
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
