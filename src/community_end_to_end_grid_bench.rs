use crate::community_routing::cosine;
use crate::community_routing_bench_fixture::load_fixture;
use crate::community_subcentroid_routing::{build_subcentroids, route_subcentroids};
use crate::community_traversal_bench_fixture::{adjacency, ranked_targets};
use crate::community_traversal_bench_support::{traverse, TraversalPolicy};
use crate::{CommunityId, MemoryId};
use std::collections::{HashMap, HashSet};

const ROUTE_KS: [usize; 5] = [2, 3, 4, 5, 6];
const BUDGETS: [usize; 3] = [16, 32, 64];
const SEEDS: usize = 4;

#[test]
#[ignore = "end-to-end community routing operating-point grid"]
fn community_end_to_end_operating_point_benchmark() {
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
    let router = build_subcentroids(&fixture, &training, 4);
    let graph = adjacency(&fixture);
    println!("k,route_recall,seed_overlap,vector_work,budget,global_support,tiered_support,communities,crossings");

    for k in ROUTE_KS {
        let mut queries = 0_usize;
        let mut route_hits = 0_usize;
        let mut route_targets = 0_usize;
        let mut seed_hits = 0_usize;
        let mut fine_candidates = 0_usize;
        let mut global_candidates = 0_usize;
        let mut support_total = 0_usize;
        let mut global_hits = [0_usize; 3];
        let mut tiered_hits = [0_usize; 3];
        let mut communities = [0_usize; 3];
        let mut crossings = [0_usize; 3];

        for probe in fixture
            .semantic
            .iter()
            .filter(|probe| held_out.contains(&probe.query))
        {
            let Some(support) = graph.get(&probe.query) else {
                continue;
            };
            if support.is_empty() {
                continue;
            }
            let query = fixture.vectors.get(&probe.query).unwrap();
            let baseline = ranked_targets(&fixture, probe.query, &probe.targets);
            let global_seeds: Vec<_> = baseline.iter().copied().take(SEEDS).collect();
            let selected: HashSet<_> = route_subcentroids(&router, query, k).into_iter().collect();
            route_hits += probe
                .targets
                .iter()
                .filter(|target| target_admitted(&fixture.memberships, **target, &selected))
                .count();
            route_targets += probe.targets.len();
            let candidates: Vec<_> = fixture
                .vectors
                .keys()
                .copied()
                .filter(|id| *id != probe.query)
                .filter(|id| target_admitted(&fixture.memberships, *id, &selected))
                .collect();
            let tiered_seeds = fine_seeds(&fixture.vectors, query, &candidates, SEEDS);
            seed_hits += tiered_seeds
                .iter()
                .filter(|seed| global_seeds.contains(seed))
                .count();
            fine_candidates += candidates.len();
            global_candidates += fixture.vectors.len().saturating_sub(1);
            support_total += support.len();
            queries += 1;

            for (slot, budget) in BUDGETS.into_iter().enumerate() {
                let global = traverse(
                    &fixture,
                    &graph,
                    &global_seeds,
                    TraversalPolicy::Graph,
                    budget,
                );
                let tiered = traverse(
                    &fixture,
                    &graph,
                    &tiered_seeds,
                    TraversalPolicy::CommunityWithinDepth,
                    budget,
                );
                global_hits[slot] += support
                    .iter()
                    .filter(|id| global.visited.contains(id))
                    .count();
                tiered_hits[slot] += support
                    .iter()
                    .filter(|id| tiered.visited.contains(id))
                    .count();
                communities[slot] += tiered.communities;
                crossings[slot] += tiered.boundary_crossings;
            }
        }

        let route_recall = route_hits as f64 / route_targets.max(1) as f64;
        let seed_overlap = seed_hits as f64 / (queries.max(1) * SEEDS) as f64;
        let vector_work =
            (fine_candidates + queries * router.len()) as f64 / global_candidates.max(1) as f64;
        for (slot, budget) in BUDGETS.into_iter().enumerate() {
            println!(
                "{k},{route_recall:.4},{seed_overlap:.4},{vector_work:.4},{budget},{:.4},{:.4},{:.2},{:.2}",
                global_hits[slot] as f64 / support_total.max(1) as f64,
                tiered_hits[slot] as f64 / support_total.max(1) as f64,
                communities[slot] as f64 / queries.max(1) as f64,
                crossings[slot] as f64 / queries.max(1) as f64,
            );
        }
    }
}

fn target_admitted(
    memberships: &HashMap<MemoryId, CommunityId>,
    target: MemoryId,
    selected: &HashSet<CommunityId>,
) -> bool {
    memberships
        .get(&target)
        .is_none_or(|community| selected.contains(community))
}

fn fine_seeds(
    vectors: &HashMap<MemoryId, Vec<f32>>,
    query: &[f32],
    candidates: &[MemoryId],
    limit: usize,
) -> Vec<MemoryId> {
    let mut ranked: Vec<_> = candidates
        .iter()
        .filter_map(|id| cosine(query, vectors.get(id)?).map(|score| (*id, score)))
        .collect();
    ranked.sort_by(|left, right| {
        right
            .1
            .total_cmp(&left.1)
            .then_with(|| left.0 .0.cmp(&right.0 .0))
    });
    ranked.into_iter().take(limit).map(|(id, _)| id).collect()
}
