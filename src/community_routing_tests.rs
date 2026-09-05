use crate::community_routing::{RepresentativeStrategy, cosine, routing_profiles};
use crate::{
    Community, CommunityId, CommunitySnapshot, GraphRelation, GraphRelationKind,
    GraphRelationOrigin, MemoryId,
};
use std::collections::HashMap;

#[test]
fn representative_selection_is_deterministic_and_bounded() {
    let ids = (0..5).map(memory).collect::<Vec<_>>();
    let snapshot = snapshot(ids.clone());
    let vectors = HashMap::from([
        (ids[0], vec![1.0, 0.0]),
        (ids[1], vec![0.95, 0.05]),
        (ids[2], vec![0.8, 0.2]),
        (ids[3], vec![0.1, 0.9]),
        (ids[4], vec![0.0, 1.0]),
    ]);
    let relations = vec![
        relation(ids[0], ids[1], GraphRelationKind::Topical),
        relation(ids[1], ids[2], GraphRelationKind::Factual),
        relation(ids[1], ids[3], GraphRelationKind::Causal),
        relation(ids[1], ids[3], GraphRelationKind::References),
    ];

    for strategy in [
        RepresentativeStrategy::SingleMedoid,
        RepresentativeStrategy::Diverse,
        RepresentativeStrategy::StructuralCentral,
    ] {
        let first = routing_profiles(&snapshot, "owner", &vectors, &relations, strategy, 3);
        let second = routing_profiles(&snapshot, "owner", &vectors, &relations, strategy, 3);
        assert_eq!(first, second);
        assert_eq!(first.len(), 1);
        let expected = if strategy == RepresentativeStrategy::SingleMedoid {
            1
        } else {
            3
        };
        assert_eq!(first[0].representatives.len(), expected);
    }

    let central = routing_profiles(
        &snapshot,
        "owner",
        &vectors,
        &relations,
        RepresentativeStrategy::StructuralCentral,
        2,
    );
    assert_eq!(central[0].representatives[0].memory_id, ids[1]);
}

#[test]
fn routing_profiles_only_reference_existing_vectors() {
    let ids = (0..3).map(memory).collect::<Vec<_>>();
    let snapshot = snapshot(ids.clone());
    let vectors = HashMap::from([(ids[0], vec![1.0, 0.0]), (ids[2], vec![0.0, 1.0])]);
    let profiles = routing_profiles(
        &snapshot,
        "owner",
        &vectors,
        &[],
        RepresentativeStrategy::Diverse,
        8,
    );
    let selected: Vec<_> = profiles[0]
        .representatives
        .iter()
        .map(|reference| reference.memory_id)
        .collect();
    assert_eq!(selected, vec![ids[0], ids[2]]);
}

#[test]
fn cosine_rejects_invalid_shapes() {
    assert_eq!(cosine(&[1.0, 0.0], &[1.0, 0.0]), Some(1.0));
    assert_eq!(cosine(&[], &[]), None);
    assert_eq!(cosine(&[1.0], &[1.0, 2.0]), None);
    assert_eq!(cosine(&[0.0, 0.0], &[1.0, 0.0]), None);
}

fn snapshot(members: Vec<MemoryId>) -> CommunitySnapshot {
    CommunitySnapshot {
        generation: 1,
        derived_graph_version: 1,
        algorithm_version: 2,
        seed: 1,
        resolution: 1.0,
        quality: 0.0,
        communities: vec![Community {
            id: CommunityId([7; 32]),
            members,
        }],
    }
}

fn relation(source: MemoryId, target: MemoryId, kind: GraphRelationKind) -> GraphRelation {
    GraphRelation {
        source,
        target,
        kind,
        active: true,
        origin: GraphRelationOrigin::Dream,
        global_version: 1,
        graph_version: 1,
    }
}

fn memory(index: usize) -> MemoryId {
    let mut bytes = [0_u8; 32];
    bytes[..8].copy_from_slice(&(index as u64).to_le_bytes());
    MemoryId(bytes)
}
