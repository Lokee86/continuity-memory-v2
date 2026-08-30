use crate::{CommunityId, GraphRelation, MemoryId};
use std::collections::{HashMap, HashSet};

#[derive(Clone, Copy)]
struct FrontierItem {
    node: MemoryId,
    depth: usize,
    boundaries: usize,
    sequence: u64,
}

pub(crate) fn traverse(
    relations: &[GraphRelation],
    memberships: &HashMap<MemoryId, CommunityId>,
    searchable: &HashSet<MemoryId>,
    seeds: &[MemoryId],
    budget: usize,
    max_depth: usize,
    community_within_depth: bool,
) -> Vec<MemoryId> {
    let adjacency = adjacency(relations, searchable);
    let mut frontier = Vec::new();
    let mut best = HashMap::<MemoryId, (usize, usize)>::new();
    let mut visited = HashSet::new();
    let mut ordered = Vec::new();
    let mut sequence = 0_u64;

    for &seed in seeds {
        if !searchable.contains(&seed) {
            continue;
        }
        best.insert(seed, (0, 0));
        frontier.push(FrontierItem {
            node: seed,
            depth: 0,
            boundaries: 0,
            sequence,
        });
        sequence += 1;
    }

    while ordered.len() < budget && !frontier.is_empty() {
        let index = frontier_index(&frontier, community_within_depth);
        let item = frontier.swap_remove(index);
        if visited.contains(&item.node)
            || best.get(&item.node) != Some(&(item.boundaries, item.depth))
        {
            continue;
        }
        visited.insert(item.node);
        ordered.push(item.node);
        if item.depth == max_depth {
            continue;
        }
        for &neighbor in adjacency.get(&item.node).into_iter().flatten() {
            if visited.contains(&neighbor) {
                continue;
            }
            let crossed = memberships.get(&item.node) != memberships.get(&neighbor);
            let candidate = (item.boundaries + usize::from(crossed), item.depth + 1);
            if best
                .get(&neighbor)
                .is_none_or(|current| candidate < *current)
            {
                best.insert(neighbor, candidate);
                frontier.push(FrontierItem {
                    node: neighbor,
                    depth: candidate.1,
                    boundaries: candidate.0,
                    sequence,
                });
                sequence += 1;
            }
        }
    }
    ordered
}

fn adjacency(
    relations: &[GraphRelation],
    searchable: &HashSet<MemoryId>,
) -> HashMap<MemoryId, Vec<MemoryId>> {
    let mut values = HashMap::<MemoryId, HashSet<MemoryId>>::new();
    for relation in relations {
        if !relation.active
            || !searchable.contains(&relation.source)
            || !searchable.contains(&relation.target)
        {
            continue;
        }
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

fn frontier_index(frontier: &[FrontierItem], community_within_depth: bool) -> usize {
    frontier
        .iter()
        .enumerate()
        .min_by_key(|(_, item)| {
            if community_within_depth {
                (item.depth, item.boundaries, item.sequence)
            } else {
                (item.depth, 0, item.sequence)
            }
        })
        .map(|(index, _)| index)
        .expect("non-empty retrieval frontier")
}
