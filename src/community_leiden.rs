use crate::community_store::community_id;
use crate::graph_store::GraphStore;
use crate::{
    COMMUNITY_ALGORITHM_VERSION, COMMUNITY_LEIDEN_RESOLUTION, COMMUNITY_LEIDEN_SEED, Community,
    CommunityError, CommunitySnapshot, MemoryId,
};
use leiden_rs::{GraphDataBuilder, Leiden, LeidenConfig, QualityType};
use std::collections::{BTreeSet, HashMap};

pub(crate) fn full_graph_snapshot(
    graph: &GraphStore,
    owner_uuid: [u8; 16],
    generation: u64,
) -> Result<CommunitySnapshot, CommunityError> {
    let graph_version = graph.graph_version();
    let nodes = graph.node_memory_ids();
    if nodes.is_empty() {
        return Ok(snapshot(generation, graph_version, 0.0, Vec::new()));
    }

    let edges = structural_edges(graph)?;
    if edges.is_empty() {
        let communities = nodes
            .into_iter()
            .map(|member| make_community(owner_uuid, vec![member]))
            .collect::<Vec<_>>();
        return Ok(snapshot(
            generation,
            graph_version,
            0.0,
            sorted(communities),
        ));
    }

    let mut builder = GraphDataBuilder::new(nodes.len());
    for (source, target) in edges {
        builder
            .add_edge(source, target, 1.0)
            .map_err(|error| CommunityError::Leiden(error.to_string()))?;
    }
    let data = builder
        .build()
        .map_err(|error| CommunityError::Leiden(error.to_string()))?;
    let config = LeidenConfig {
        resolution: COMMUNITY_LEIDEN_RESOLUTION,
        seed: Some(COMMUNITY_LEIDEN_SEED),
        quality: QualityType::Modularity,
        parallel_local_moving_threshold: None,
        parallel_aggregation_threshold: None,
        ..LeidenConfig::default()
    };
    let result = Leiden::new(config)
        .run(&data)
        .map_err(|error| CommunityError::Leiden(error.to_string()))?;

    let mut grouped = HashMap::<usize, Vec<MemoryId>>::new();
    for (node, community) in result.partition.iter() {
        grouped.entry(community).or_default().push(nodes[node]);
    }
    let communities = grouped
        .into_values()
        .map(|mut members| {
            members.sort_by_key(|member| member.0);
            make_community(owner_uuid, members)
        })
        .collect();
    Ok(snapshot(
        generation,
        graph_version,
        result.quality,
        sorted(communities),
    ))
}

pub(crate) fn structural_edges(
    graph: &GraphStore,
) -> Result<BTreeSet<(usize, usize)>, CommunityError> {
    let mut edges = BTreeSet::new();
    for relation in graph.active_relations() {
        let source = graph.node_id(relation.source)?.0 as usize;
        let target = graph.node_id(relation.target)?.0 as usize;
        let pair = if source < target {
            (source, target)
        } else {
            (target, source)
        };
        edges.insert(pair);
    }
    Ok(edges)
}

fn make_community(owner_uuid: [u8; 16], members: Vec<MemoryId>) -> Community {
    Community {
        id: community_id(owner_uuid, &members),
        members,
    }
}

fn sorted(mut communities: Vec<Community>) -> Vec<Community> {
    communities.sort_by_key(|community| community.id);
    communities
}

fn snapshot(
    generation: u64,
    derived_graph_version: u64,
    quality: f64,
    communities: Vec<Community>,
) -> CommunitySnapshot {
    CommunitySnapshot {
        generation,
        derived_graph_version,
        algorithm_version: COMMUNITY_ALGORITHM_VERSION,
        seed: COMMUNITY_LEIDEN_SEED,
        resolution: COMMUNITY_LEIDEN_RESOLUTION,
        quality,
        communities,
    }
}
