use crate::MemoryId;
use crate::community_routing_bench_fixture::RoutingFixture;
use crate::community_traversal_bench_fixture::{adjacency, ranked_targets};
use std::collections::{HashMap, HashSet};
use std::time::Instant;

const SEEDS: usize = 4;
const MAX_DEPTH: usize = 3;

#[derive(Clone, Copy)]
pub(crate) enum TraversalPolicy {
    Graph,
    CommunityPreferred,
    CommunityWithinDepth,
}

#[derive(Default)]
pub(crate) struct Totals {
    pub(crate) queries: usize,
    pub(crate) support_hits: usize,
    pub(crate) support_total: usize,
    pub(crate) nodes: usize,
    pub(crate) edges: usize,
    pub(crate) communities: usize,
    pub(crate) boundary_crossings: usize,
    pub(crate) tokens: usize,
    pub(crate) elapsed_ns: u128,
}

#[derive(Clone, Copy)]
struct FrontierItem {
    node: MemoryId,
    depth: usize,
    boundaries: usize,
    sequence: u64,
    crossed_boundary: bool,
}

pub(crate) struct TraversalResult {
    pub(crate) visited: HashSet<MemoryId>,
    pub(crate) edges: usize,
    pub(crate) communities: usize,
    pub(crate) boundary_crossings: usize,
    pub(crate) tokens: usize,
}

pub(crate) fn evaluate(fixture: &RoutingFixture, policy: TraversalPolicy, budget: usize) -> Totals {
    let adjacency = adjacency(fixture);
    let mut totals = Totals::default();
    for probe in &fixture.semantic {
        let Some(support) = adjacency.get(&probe.query) else {
            continue;
        };
        if support.is_empty() {
            continue;
        }
        let seeds = ranked_targets(fixture, probe.query, &probe.targets)
            .into_iter()
            .take(SEEDS)
            .collect::<Vec<_>>();
        let started = Instant::now();
        let result = traverse(fixture, &adjacency, &seeds, policy, budget);
        totals.elapsed_ns += started.elapsed().as_nanos();
        totals.queries += 1;
        totals.support_total += support.len();
        totals.support_hits += support
            .iter()
            .filter(|memory| result.visited.contains(memory))
            .count();
        totals.nodes += result.visited.len();
        totals.edges += result.edges;
        totals.boundary_crossings += result.boundary_crossings;
        totals.communities += result.communities;
        totals.tokens += result.tokens;
    }
    totals
}

pub(crate) fn traverse(
    fixture: &RoutingFixture,
    adjacency: &HashMap<MemoryId, Vec<MemoryId>>,
    seeds: &[MemoryId],
    policy: TraversalPolicy,
    budget: usize,
) -> TraversalResult {
    let mut frontier = Vec::new();
    let mut best = HashMap::<MemoryId, (usize, usize)>::new();
    let mut visited = HashSet::new();
    let mut sequence = 0_u64;
    for &seed in seeds {
        best.insert(seed, (0, 0));
        frontier.push(FrontierItem {
            node: seed,
            depth: 0,
            boundaries: 0,
            sequence,
            crossed_boundary: false,
        });
        sequence += 1;
    }
    let mut edges = 0_usize;
    let mut boundary_crossings = 0_usize;
    while visited.len() < budget && !frontier.is_empty() {
        let index = frontier_index(&frontier, policy);
        let item = frontier.swap_remove(index);
        if visited.contains(&item.node)
            || best.get(&item.node) != Some(&(item.boundaries, item.depth))
        {
            continue;
        }
        visited.insert(item.node);
        boundary_crossings += usize::from(item.crossed_boundary);
        if item.depth == MAX_DEPTH {
            continue;
        }
        for &neighbor in adjacency.get(&item.node).into_iter().flatten() {
            edges += 1;
            if visited.contains(&neighbor) {
                continue;
            }
            let crossed = fixture.memberships.get(&item.node) != fixture.memberships.get(&neighbor);
            let candidate = (item.boundaries + usize::from(crossed), item.depth + 1);
            if best
                .get(&neighbor)
                .map_or(true, |current| candidate < *current)
            {
                best.insert(neighbor, candidate);
                frontier.push(FrontierItem {
                    node: neighbor,
                    depth: candidate.1,
                    boundaries: candidate.0,
                    sequence,
                    crossed_boundary: crossed,
                });
                sequence += 1;
            }
        }
    }
    let communities = visited
        .iter()
        .filter_map(|memory| fixture.memberships.get(memory))
        .collect::<HashSet<_>>()
        .len();
    let tokens = visited
        .iter()
        .filter_map(|memory| fixture.memories.get(memory))
        .map(|memory| {
            memory.title.split_whitespace().count() + memory.content.split_whitespace().count()
        })
        .sum();
    TraversalResult {
        visited,
        edges,
        communities,
        boundary_crossings,
        tokens,
    }
}

fn frontier_index(frontier: &[FrontierItem], policy: TraversalPolicy) -> usize {
    frontier
        .iter()
        .enumerate()
        .min_by_key(|(_, item)| match policy {
            TraversalPolicy::Graph => (item.depth, 0, item.sequence),
            TraversalPolicy::CommunityPreferred => (item.boundaries, item.depth, item.sequence),
            TraversalPolicy::CommunityWithinDepth => (item.depth, item.boundaries, item.sequence),
        })
        .map(|(index, _)| index)
        .unwrap()
}

pub(crate) fn policy_name(policy: TraversalPolicy) -> &'static str {
    match policy {
        TraversalPolicy::Graph => "graph",
        TraversalPolicy::CommunityPreferred => "community-first",
        TraversalPolicy::CommunityWithinDepth => "community-within-depth",
    }
}
