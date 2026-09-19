use crate::community_test_support::{edge, path, phy_memory, rel_episode, rel_memory};
use crate::memory_retrieval_traversal::traverse;
use crate::{
    Cva, EmbeddingEndpoint, EmbeddingMode, GraphRelation, GraphRelationKind, GraphRelationOrigin,
    MemoryId, MemoryRetrievalConfig, MemoryRetrievalError, MemoryRetrievalMode, Phylactery,
    SimulatedEmbeddingEndpoint, VectorNormalization,
};
use std::collections::{HashMap, HashSet};

#[test]
fn rel_retrieval_routes_current_communities_and_falls_back_without_them() {
    let mut rel = Cva::create_project(path("retrieval.prj.rel")).unwrap();
    let episode = rel_episode(&mut rel);
    let a = rel_memory(&mut rel, &episode, "alpha-a");
    let b = rel_memory(&mut rel, &episode, "alpha-b");
    let c = rel_memory(&mut rel, &episode, "alpha-c");
    let d = rel_memory(&mut rel, &episode, "beta-d");
    let e = rel_memory(&mut rel, &episode, "beta-e");
    let f = rel_memory(&mut rel, &episode, "beta-f");
    let orphan = rel_memory(&mut rel, &episode, "orphan");
    rel.set_memory_relations(
        &[
            edge(a, b),
            edge(b, c),
            edge(c, a),
            edge(d, e),
            edge(e, f),
            edge(f, d),
            edge(c, d),
        ],
        0,
    )
    .unwrap();

    let endpoint = SimulatedEmbeddingEndpoint::new(8, VectorNormalization::L2, 42);
    let profile = rel.establish_compatibility_profile(&endpoint).unwrap();
    rel.build_missing_memory_vectors(profile.id, &endpoint)
        .unwrap();
    let query = endpoint
        .embed(EmbeddingMode::Query, &["memory alpha-a".into()])
        .unwrap()
        .remove(0);

    let fallback = rel
        .retrieve_memories(profile.id, &query, MemoryRetrievalConfig::default())
        .unwrap();
    assert_eq!(fallback.mode_used, MemoryRetrievalMode::GlobalExact);
    assert!(fallback.used_global_fallback);
    assert_eq!(fallback.memory_vectors_scored, 7);

    rel.refresh_communities_leiden().unwrap();
    let config = MemoryRetrievalConfig {
        community_limit: 1,
        ..MemoryRetrievalConfig::default()
    };
    let index = rel
        .build_memory_retrieval_index(profile.id, config.subcentroids_per_community)
        .unwrap();
    let routed = rel
        .retrieve_memories_with_index(&index, &query, config)
        .unwrap();
    assert_eq!(routed.mode_used, MemoryRetrievalMode::CommunityRouted);
    assert!(!routed.used_global_fallback);
    assert_eq!(routed.selected_communities.len(), 1);
    assert!(routed.routing_vectors_scored > 0);
    assert!(routed.memory_vectors_scored < routed.global_memory_vectors);
    assert_eq!(routed.global_memory_vectors, 7);
    assert!(
        routed.memory_vectors_scored >= 4,
        "orphan residual lane must remain admitted"
    );
    assert!(!routed.memories.is_empty());
    assert!(routed.memories.len() <= MemoryRetrievalConfig::default().traversal_budget);
    assert!(rel.memory_ids().contains(&orphan));

    let global = rel
        .retrieve_memories(
            profile.id,
            &query,
            MemoryRetrievalConfig {
                mode: MemoryRetrievalMode::GlobalExact,
                ..MemoryRetrievalConfig::default()
            },
        )
        .unwrap();
    assert_eq!(global.mode_used, MemoryRetrievalMode::GlobalExact);
    assert!(!global.used_global_fallback);
    assert!(global.selected_communities.is_empty());
    assert_eq!(global.memory_vectors_scored, global.global_memory_vectors);

    rel.set_memory_relation(a, e, GraphRelationKind::Factual, true, rel.graph_version())
        .unwrap();
    assert!(matches!(
        rel.retrieve_memories_with_index(&index, &query, config),
        Err(MemoryRetrievalError::StaleIndex)
    ));
}

#[test]
fn phylactery_retrieval_uses_owner_local_community_routing() {
    let mut phy = Phylactery::create(path("retrieval.phy")).unwrap();
    let a = phy_memory(&mut phy, "a");
    let b = phy_memory(&mut phy, "b");
    let c = phy_memory(&mut phy, "c");
    let d = phy_memory(&mut phy, "d");
    phy.set_memory_relations(&[edge(a, b), edge(c, d)], 0)
        .unwrap();

    let endpoint = SimulatedEmbeddingEndpoint::new(8, VectorNormalization::L2, 9);
    let profile = phy.establish_compatibility_profile(&endpoint).unwrap();
    phy.build_missing_memory_vectors(profile.id, &endpoint)
        .unwrap();
    phy.refresh_communities_leiden().unwrap();
    let query = endpoint
        .embed(EmbeddingMode::Query, &["user memory a".into()])
        .unwrap()
        .remove(0);
    let result = phy
        .retrieve_memories(
            profile.id,
            &query,
            MemoryRetrievalConfig {
                community_limit: 1,
                ..MemoryRetrievalConfig::default()
            },
        )
        .unwrap();

    assert_eq!(result.mode_used, MemoryRetrievalMode::CommunityRouted);
    assert_eq!(result.selected_communities.len(), 1);
    assert!(result.memory_vectors_scored < result.global_memory_vectors);
}

#[test]
fn community_traversal_prefers_same_region_only_within_equal_depth() {
    let a = id(1);
    let cross = id(2);
    let local = id(3);
    let deep_local = id(4);
    let relations = vec![
        relation(a, cross),
        relation(a, local),
        relation(local, deep_local),
    ];
    let community_a = crate::CommunityId([10; 32]);
    let community_b = crate::CommunityId([20; 32]);
    let memberships = HashMap::from([
        (a, community_a),
        (local, community_a),
        (deep_local, community_a),
        (cross, community_b),
    ]);
    let searchable = HashSet::from([a, cross, local, deep_local]);

    let global = traverse(&relations, &memberships, &searchable, &[a], 4, 3, false);
    let routed = traverse(&relations, &memberships, &searchable, &[a], 4, 3, true);

    assert_eq!(global, vec![a, cross, local, deep_local]);
    assert_eq!(routed, vec![a, local, cross, deep_local]);
    assert_eq!(
        routed[3], deep_local,
        "community preference must not beat graph depth"
    );
}

fn id(value: u8) -> MemoryId {
    MemoryId([value; 32])
}

fn relation(source: MemoryId, target: MemoryId) -> GraphRelation {
    GraphRelation {
        source,
        target,
        kind: GraphRelationKind::Topical,
        active: true,
        origin: GraphRelationOrigin::Dream,
        global_version: 1,
        graph_version: 1,
    }
}
