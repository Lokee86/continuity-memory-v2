use crate::community_test_support::{edge, path, rel_episode, rel_memory};
use crate::{
    Cva, EmbeddingEndpoint, EmbeddingMode, EntityDraft, MemoryRetrievalConfig,
    MemoryRetrievalError, PackedVectors, ScalarType, SimulatedEmbeddingEndpoint,
    VectorNormalization, VectorSchema,
};

#[test]
fn cached_index_rejects_vector_population_changes_without_memory_mutation() {
    let mut rel = Cva::create_project(path("retrieval-vector-stale.prj.rel")).unwrap();
    let episode = rel_episode(&mut rel);
    let a = rel_memory(&mut rel, &episode, "a");
    let b = rel_memory(&mut rel, &episode, "b");
    rel.set_memory_relations(&[edge(a, b)], 0).unwrap();

    let endpoint = SimulatedEmbeddingEndpoint::new(8, VectorNormalization::L2, 77);
    let profile = rel.establish_compatibility_profile(&endpoint).unwrap();
    bind_one(&mut rel, profile.id, &endpoint, a, "memory a");
    rel.refresh_communities_leiden().unwrap();

    let config = MemoryRetrievalConfig::default();
    let index = rel
        .build_memory_retrieval_index(profile.id, config.subcentroids_per_community)
        .unwrap();
    assert_eq!(index.vector_bindings, 1);
    let memory_version = rel.memory_version();
    let graph_version = rel.graph_version();
    let community_generation = rel.community_snapshot().unwrap().generation;

    bind_one(&mut rel, profile.id, &endpoint, b, "memory b");
    assert_eq!(rel.memory_version(), memory_version);
    assert_eq!(rel.graph_version(), graph_version);
    assert_eq!(
        rel.community_snapshot().unwrap().generation,
        community_generation
    );

    let query = endpoint
        .embed(EmbeddingMode::Query, &["memory a".into()])
        .unwrap()
        .remove(0);
    assert!(matches!(
        rel.retrieve_memories_with_index(&index, &query, config),
        Err(MemoryRetrievalError::StaleIndex)
    ));
}

#[test]
fn cached_index_survives_entity_only_graph_mutation() {
    let mut rel = Cva::create_project(path("retrieval-entity-graph.prj.rel")).unwrap();
    let episode = rel_episode(&mut rel);
    let a = rel_memory(&mut rel, &episode, "a");
    let b = rel_memory(&mut rel, &episode, "b");
    rel.set_memory_relations(&[edge(a, b)], 0).unwrap();

    let endpoint = SimulatedEmbeddingEndpoint::new(8, VectorNormalization::L2, 78);
    let profile = rel.establish_compatibility_profile(&endpoint).unwrap();
    bind_one(&mut rel, profile.id, &endpoint, a, "memory a");
    bind_one(&mut rel, profile.id, &endpoint, b, "memory b");
    rel.refresh_communities_leiden().unwrap();

    let config = MemoryRetrievalConfig::default();
    let index = rel
        .build_memory_retrieval_index(profile.id, config.subcentroids_per_community)
        .unwrap();
    let graph_version = rel.graph_version();
    let memory_graph_version = rel.memory_graph_version();

    let (entity, _) = rel
        .publish_entity(
            None,
            0,
            EntityDraft {
                canonical_name: "Reliquary".into(),
                aliases: vec![],
                kind: "project".into(),
                summary: "Project referent.".into(),
                mutation_id: "retrieval-entity".into(),
                created_at_ns: 1,
                updated_at_ns: 1,
            },
        )
        .unwrap();
    rel.set_entity_association(a, entity.id, true, graph_version)
        .unwrap();

    assert_eq!(rel.graph_version(), graph_version + 1);
    assert_eq!(rel.memory_graph_version(), memory_graph_version);
    assert!(rel.community_stats().current);

    let query = endpoint
        .embed(EmbeddingMode::Query, &["memory a".into()])
        .unwrap()
        .remove(0);
    rel.retrieve_memories_with_index(&index, &query, config)
        .unwrap();
}

fn bind_one(
    rel: &mut Cva,
    profile_id: crate::CompatibilityProfileId,
    endpoint: &impl EmbeddingEndpoint,
    memory_id: crate::MemoryId,
    text: &str,
) {
    let vector = endpoint
        .embed(EmbeddingMode::Document, &[text.to_string()])
        .unwrap()
        .remove(0);
    let schema = VectorSchema::new(vector.len() as u32, ScalarType::F32).unwrap();
    let mut bytes = Vec::with_capacity(vector.len() * 4);
    for value in vector {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    let packed = PackedVectors::from_bytes(schema, bytes).unwrap();
    let packed_id = rel.put_packed_vectors(packed).unwrap();
    let body_id = rel.memory_body_id(memory_id).unwrap();
    rel.put_memory_vectors(profile_id, packed_id, vec![body_id])
        .unwrap();
}
