use crate::{
    Cva, EntityId, EpisodeBoundary, EpisodeConfig, EpisodeOrigin, GraphDirection, GraphError,
    GraphRelationChange, GraphRelationKind, GraphRelationOrigin, MemoryDraft, MemoryId,
    SemanticNodeKind, SemanticNodeRef,
};
use std::{fs, path::PathBuf};

fn path(name: &str) -> PathBuf {
    let n = uuid::Uuid::new_v4();
    let dir = std::env::temp_dir().join(format!("continuity-graph-{n}"));
    fs::create_dir_all(&dir).unwrap();
    dir.join(name)
}

fn episode(cva: &mut Cva) -> crate::Episode {
    cva.append_node(
        "u".into(),
        "c".into(),
        None,
        "user".into(),
        10,
        "connect memories",
    )
    .unwrap();
    cva.append_node(
        "a".into(),
        "c".into(),
        Some("u".into()),
        "assistant".into(),
        20,
        "ok",
    )
    .unwrap();
    cva.materialize_path_episodes(
        "c",
        "a",
        EpisodeConfig::default(),
        EpisodeOrigin::Live,
        Some((EpisodeBoundary::Inactivity, 30)),
    )
    .unwrap()
    .created
    .into_iter()
    .next()
    .unwrap()
}

fn memory(cva: &mut Cva, episode: &crate::Episode, id: &str) -> MemoryId {
    cva.publish_memory(
        None,
        0,
        MemoryDraft {
            category: "fact".into(),
            memory_type: "project".into(),
            authority_kind: "unknown".into(),
            temporal_status: "unknown".into(),
            title: id.into(),
            content: format!("memory {id}"),
            scope: "private".into(),
            lifecycle_state: "extracted".into(),
            archived: false,
            superseded_by: None,
            parent_id: None,
            source_node_id: Some("u".into()),
            content_source_conversation_id: None,
            content_source_node_id: None,
            grounding_source_conversation_id: None,
            grounding_source_node_id: None,
            source_episode_id: Some(episode.id),
            source_time_ns: None,
            mutation_id: id.into(),
            created_at_ns: 30,
            updated_at_ns: 30,
        },
    )
    .unwrap()
    .0
    .id
}

#[test]
fn semantic_node_identity_distinguishes_node_kinds() {
    let raw = [7_u8; 32];
    let memory = SemanticNodeRef::memory(MemoryId(raw));
    let entity = SemanticNodeRef::entity(EntityId(raw));

    assert_ne!(memory, entity);
    assert_eq!(memory.kind, SemanticNodeKind::Memory);
    assert_eq!(entity.kind, SemanticNodeKind::Entity);
    assert_eq!(memory.as_memory(), Some(MemoryId(raw)));
    assert_eq!(entity.as_entity(), Some(EntityId(raw)));
}

#[test]
fn graph_node_codec_reads_legacy_memory_and_typed_entity_nodes() {
    use crate::graph_codec::{GraphNodePayload, decode_node, encode_node};
    use arcana::NodeId;

    let memory = GraphNodePayload {
        semantic_node: SemanticNodeRef::memory(MemoryId([1; 32])),
        node_id: NodeId(3),
    };
    let memory_bytes = encode_node(memory);
    assert_eq!(&memory_bytes[..8], b"CVAGNODE");
    assert_eq!(decode_node(&memory_bytes).unwrap(), Some(memory));

    let entity = GraphNodePayload {
        semantic_node: SemanticNodeRef::entity(EntityId([2; 32])),
        node_id: NodeId(4),
    };
    let entity_bytes = encode_node(entity);
    assert_eq!(&entity_bytes[..8], b"CVAGNOD2");
    assert_eq!(decode_node(&entity_bytes).unwrap(), Some(entity));
}

#[test]
fn graph_is_oriented_traversable_retractable_and_reopenable() {
    let file = path("graph.cva");
    let mut cva = Cva::create(&file).unwrap();
    let ep = episode(&mut cva);
    let a = memory(&mut cva, &ep, "a");
    let b = memory(&mut cva, &ep, "b");
    let c = memory(&mut cva, &ep, "c");

    cva.set_memory_relation(a, b, GraphRelationKind::Factual, true, 0)
        .unwrap()
        .unwrap();
    cva.set_memory_relation(b, c, GraphRelationKind::Causal, true, 1)
        .unwrap()
        .unwrap();
    assert!(
        cva.set_memory_relation(b, c, GraphRelationKind::Causal, true, 2)
            .unwrap()
            .is_none()
    );
    assert_eq!(cva.graph_version(), 2);
    assert_eq!(
        cva.graph.semantic_nodes(),
        vec![
            SemanticNodeRef::memory(a),
            SemanticNodeRef::memory(b),
            SemanticNodeRef::memory(c)
        ]
    );
    assert!(
        cva.graph_relations()
            .iter()
            .all(|relation| relation.origin == GraphRelationOrigin::Dream)
    );
    assert_eq!(
        cva.graph_neighbors(a, GraphDirection::Outgoing).unwrap()[0].memory_id,
        b
    );
    assert_eq!(
        cva.graph_neighbors(b, GraphDirection::Incoming).unwrap()[0].memory_id,
        a
    );
    let chain = cva.shortest_memory_path(a, c, 2).unwrap().unwrap();
    assert_eq!(chain.memories, vec![a, b, c]);
    assert_eq!(
        chain.relations,
        vec![GraphRelationKind::Factual, GraphRelationKind::Causal]
    );

    cva.set_memory_relation(a, b, GraphRelationKind::Factual, false, 2)
        .unwrap()
        .unwrap();
    assert!(cva.shortest_memory_path(a, c, 2).unwrap().is_none());
    cva.sync().unwrap();
    drop(cva);

    let reopened = Cva::open(&file).unwrap();
    assert_eq!(reopened.graph_version(), 3);
    assert_eq!(reopened.graph_stats().nodes, 3);
    assert_eq!(reopened.graph_stats().relation_mutations, 3);
    assert_eq!(reopened.graph_stats().active_relations, 1);
    assert!(
        reopened
            .graph_neighbors(a, GraphDirection::Outgoing)
            .unwrap()
            .is_empty()
    );
}

#[test]
fn relation_origin_can_be_user_authored_and_survives_reopen() {
    let file = path("origin.cva");
    let mut cva = Cva::create(&file).unwrap();
    let ep = episode(&mut cva);
    let a = memory(&mut cva, &ep, "a");
    let b = memory(&mut cva, &ep, "b");

    let dream = cva
        .set_memory_relation(a, b, GraphRelationKind::Factual, true, 0)
        .unwrap()
        .unwrap();
    assert_eq!(dream.origin, GraphRelationOrigin::Dream);

    let user = cva
        .set_memory_relation_with_origin(
            a,
            b,
            GraphRelationKind::Factual,
            true,
            GraphRelationOrigin::User,
            1,
        )
        .unwrap()
        .unwrap();
    assert_eq!(user.origin, GraphRelationOrigin::User);
    assert_eq!(cva.graph_version(), 2);
    cva.sync().unwrap();
    drop(cva);

    let reopened = Cva::open(&file).unwrap();
    let relation = reopened.graph_relations().into_iter().next().unwrap();
    assert_eq!(relation.origin, GraphRelationOrigin::User);
    assert_eq!(reopened.graph_stats().relation_mutations, 2);
}

#[test]
fn legacy_relation_payload_defaults_origin_to_dream() {
    let mut bytes = [0_u8; 75];
    bytes[..8].copy_from_slice(b"CVAGMUT1");
    bytes[8..40].copy_from_slice(&[1; 32]);
    bytes[40..72].copy_from_slice(&[2; 32]);
    bytes[72..74].copy_from_slice(&GraphRelationKind::Factual.code().to_le_bytes());
    bytes[74] = 1;

    let decoded = crate::graph_codec::decode_mutation(&bytes)
        .unwrap()
        .unwrap();
    assert_eq!(decoded.origin, GraphRelationOrigin::Dream);
}

#[test]
fn graph_enforces_version_and_endpoint_rules() {
    let file = path("rules.cva");
    let mut cva = Cva::create(&file).unwrap();
    let ep = episode(&mut cva);
    let a = memory(&mut cva, &ep, "a");
    let b = memory(&mut cva, &ep, "b");
    cva.set_memory_relation(a, b, GraphRelationKind::Topical, true, 0)
        .unwrap();
    assert!(matches!(
        cva.set_memory_relation(a, b, GraphRelationKind::Topical, false, 0),
        Err(GraphError::RevisionConflict { .. })
    ));
    assert!(matches!(
        cva.set_memory_relation(a, a, GraphRelationKind::Factual, true, 1),
        Err(GraphError::SelfRelation)
    ));
}

#[test]
fn relation_batch_is_one_versioned_transaction_and_reopens_atomically() {
    let file = path("batch.cva");
    let mut cva = Cva::create(&file).unwrap();
    let ep = episode(&mut cva);
    let a = memory(&mut cva, &ep, "a");
    let b = memory(&mut cva, &ep, "b");

    let published = cva
        .set_memory_relations(
            &[
                GraphRelationChange {
                    source: a,
                    target: b,
                    kind: GraphRelationKind::Topical,
                    active: true,
                },
                GraphRelationChange {
                    source: b,
                    target: a,
                    kind: GraphRelationKind::Topical,
                    active: true,
                },
            ],
            0,
        )
        .unwrap();
    assert_eq!(published.len(), 2);
    assert_eq!(published[0].graph_version, 1);
    assert_eq!(published[1].graph_version, 1);
    assert_eq!(published[0].global_version, published[1].global_version);
    assert_eq!(cva.graph_version(), 1);
    assert_eq!(cva.graph_stats().relation_mutations, 2);
    cva.sync().unwrap();
    drop(cva);

    let reopened = Cva::open(&file).unwrap();
    assert_eq!(reopened.graph_version(), 1);
    assert_eq!(reopened.graph_relations().len(), 2);
    assert_eq!(reopened.graph_stats().relation_mutations, 2);
}

#[test]
fn relation_batch_rejects_duplicate_relationship_identity() {
    let file = path("batch-duplicate.cva");
    let mut cva = Cva::create(&file).unwrap();
    let ep = episode(&mut cva);
    let a = memory(&mut cva, &ep, "a");
    let b = memory(&mut cva, &ep, "b");
    let change = GraphRelationChange {
        source: a,
        target: b,
        kind: GraphRelationKind::Factual,
        active: true,
    };
    assert!(matches!(
        cva.set_memory_relations(&[change, change], 0),
        Err(GraphError::DuplicateRelationChange)
    ));
    assert_eq!(cva.graph_version(), 0);
}

#[test]
fn pre_graph_cva_state_opens_as_empty_graph() {
    let memories = crate::memory_store::MemoryStore::empty();
    let graph = crate::graph_rebuild::GraphOpenState::new()
        .finish(&memories)
        .unwrap();
    assert_eq!(graph.graph_version(), 0);
    assert_eq!(graph.stats().nodes, 0);
}
