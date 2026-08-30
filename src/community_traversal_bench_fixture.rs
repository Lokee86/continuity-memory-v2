use crate::MemoryId;
use crate::community_routing::cosine;
use crate::community_routing_bench_fixture::RoutingFixture;
use std::collections::{HashMap, HashSet};

pub(crate) fn adjacency(fixture: &RoutingFixture) -> HashMap<MemoryId, Vec<MemoryId>> {
    let mut values = HashMap::<MemoryId, HashSet<MemoryId>>::new();
    for relation in &fixture.relations {
        values
            .entry(relation.source)
            .or_default()
            .insert(relation.target);
        values
            .entry(relation.target)
            .or_default()
            .insert(relation.source);
    }
    values
        .into_iter()
        .map(|(memory, neighbors)| {
            let mut neighbors: Vec<_> = neighbors.into_iter().collect();
            neighbors.sort_by_key(|neighbor| neighbor.0);
            (memory, neighbors)
        })
        .collect()
}

pub(crate) fn ranked_targets(
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
    ranked.into_iter().map(|(memory, _)| memory).collect()
}
