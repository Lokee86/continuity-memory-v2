use crate::community_test_support::{edge, path, phy_memory, rel_episode, rel_memory};
use crate::{
    COMMUNITY_NAMING_CONTRACT_VERSION, CommunitySemanticNameSource, Cva, DreamCommunityNamer,
    DreamCommunityNamingError, Phylactery, SimulatedEmbeddingEndpoint, SimulatedGeneralEndpoint,
    VectorNormalization,
};
use serde_json::json;
use std::fs;

#[test]
fn dream_names_rel_communities_from_representatives_without_changing_partition() {
    let file = path("dream-community-names.prj.rel");
    let mut rel = Cva::create_project(&file).unwrap();
    let episode = rel_episode(&mut rel);
    let a = rel_memory(&mut rel, &episode, "alpha planning");
    let b = rel_memory(&mut rel, &episode, "alpha implementation");
    let c = rel_memory(&mut rel, &episode, "alpha testing");
    let d = rel_memory(&mut rel, &episode, "beta planning");
    let e = rel_memory(&mut rel, &episode, "beta implementation");
    let f = rel_memory(&mut rel, &episode, "beta testing");
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

    let embedding = SimulatedEmbeddingEndpoint::new(8, VectorNormalization::L2, 42);
    let profile = rel.establish_compatibility_profile(&embedding).unwrap();
    rel.build_missing_memory_vectors(profile.id, &embedding)
        .unwrap();
    let snapshot = rel.refresh_communities_leiden().unwrap();
    assert_eq!(snapshot.communities.len(), 2);
    let graph_version = rel.graph_version();
    let generation = snapshot.generation;

    let namer = DreamCommunityNamer::new(SimulatedGeneralEndpoint::new(
        "dream-namer",
        vec![json!({"name": "Alpha Work"}), json!({"name": "Beta Work"})],
    ));
    let result = namer.name_communities(&mut rel, profile.id, 8).unwrap();
    assert_eq!(result.attempted, 2);
    assert_eq!(result.named.len(), 2);
    assert_eq!(result.remaining, 0);
    assert_eq!(rel.graph_version(), graph_version);
    assert_eq!(rel.community_snapshot().unwrap().generation, generation);
    assert!(rel.community_stats().current);

    for record in rel.community_semantic_names() {
        assert_eq!(record.contract_version, COMMUNITY_NAMING_CONTRACT_VERSION);
        assert_eq!(record.source, CommunitySemanticNameSource::Dream);
        assert!(!record.name.is_empty());
        assert!(!record.representative_memories.is_empty());
        assert!(record.representative_memories.len() <= 8);
        let community = snapshot
            .communities
            .iter()
            .find(|community| community.id == record.community_id)
            .unwrap();
        assert!(
            record
                .representative_memories
                .iter()
                .all(|memory_id| community.members.contains(memory_id))
        );
    }

    rel.sync().unwrap();
    let bytes_before_noop = fs::metadata(&file).unwrap().len();
    let repeated = namer.name_communities(&mut rel, profile.id, 8).unwrap();
    assert_eq!(repeated.attempted, 0);
    assert!(repeated.named.is_empty());
    assert_eq!(repeated.remaining, 0);
    assert_eq!(fs::metadata(&file).unwrap().len(), bytes_before_noop);
    let expected_names = rel.community_semantic_names();
    drop(rel);

    let reopened = Cva::open_project(&file).unwrap();
    assert_eq!(reopened.community_semantic_names(), expected_names);
    assert_eq!(
        reopened.community_snapshot().unwrap().generation,
        generation
    );
    assert!(reopened.community_stats().current);
}

#[test]
fn dream_names_phylactery_communities_owner_locally() {
    let file = path("dream-community-names.phy");
    let mut phy = Phylactery::create(&file).unwrap();
    let a = phy_memory(&mut phy, "identity-a");
    let b = phy_memory(&mut phy, "identity-b");
    let c = phy_memory(&mut phy, "workflow-c");
    let d = phy_memory(&mut phy, "workflow-d");
    phy.set_memory_relations(&[edge(a, b), edge(c, d)], 0)
        .unwrap();

    let embedding = SimulatedEmbeddingEndpoint::new(8, VectorNormalization::L2, 9);
    let profile = phy.establish_compatibility_profile(&embedding).unwrap();
    phy.build_missing_memory_vectors(profile.id, &embedding)
        .unwrap();
    let snapshot = phy.refresh_communities_leiden().unwrap();
    assert_eq!(snapshot.communities.len(), 2);

    let namer = DreamCommunityNamer::new(SimulatedGeneralEndpoint::new(
        "dream-namer",
        vec![
            json!({"name": "Identity Context"}),
            json!({"name": "Workflow Preferences"}),
        ],
    ));
    let result = namer
        .name_phylactery_communities(&mut phy, profile.id, 8)
        .unwrap();
    assert_eq!(result.named.len(), 2);
    assert_eq!(result.remaining, 0);
    phy.sync().unwrap();
    let expected_names = phy.community_semantic_names();
    drop(phy);

    let reopened = Phylactery::open(&file).unwrap();
    assert_eq!(reopened.community_semantic_names(), expected_names);
}

#[test]
fn user_named_community_is_not_submitted_to_dream() {
    let file = path("dream-community-user-name.prj.rel");
    let mut rel = Cva::create_project(&file).unwrap();
    let episode = rel_episode(&mut rel);
    let a = rel_memory(&mut rel, &episode, "manual-a");
    let b = rel_memory(&mut rel, &episode, "manual-b");
    rel.set_memory_relations(&[edge(a, b)], 0).unwrap();

    let embedding = SimulatedEmbeddingEndpoint::new(8, VectorNormalization::L2, 17);
    let profile = rel.establish_compatibility_profile(&embedding).unwrap();
    rel.build_missing_memory_vectors(profile.id, &embedding)
        .unwrap();
    let snapshot = rel.refresh_communities_leiden().unwrap();
    let community_id = snapshot.communities[0].id;
    assert!(
        rel.set_community_name(community_id, "Manually Named Topic")
            .unwrap()
    );

    let namer = DreamCommunityNamer::new(SimulatedGeneralEndpoint::new("dream-namer", vec![]));
    let result = namer.name_communities(&mut rel, profile.id, 8).unwrap();
    assert_eq!(result.attempted, 0);
    assert!(result.named.is_empty());
    assert_eq!(result.remaining, 0);

    let name = rel.community_semantic_name(community_id).unwrap();
    assert_eq!(name.source, CommunitySemanticNameSource::User);
    assert_eq!(name.contract_version, 0);
    assert_eq!(name.name, "Manually Named Topic");
    assert!(name.representative_memories.is_empty());

    rel.sync().unwrap();
    drop(rel);
    let reopened = Cva::open_project(&file).unwrap();
    assert_eq!(
        reopened
            .community_semantic_name(community_id)
            .unwrap()
            .source,
        CommunitySemanticNameSource::User
    );
}

#[test]
fn dream_community_naming_requires_current_snapshot() {
    let mut rel = Cva::create_project(path("dream-community-name-stale.prj.rel")).unwrap();
    let episode = rel_episode(&mut rel);
    let a = rel_memory(&mut rel, &episode, "a");
    let b = rel_memory(&mut rel, &episode, "b");
    rel.set_memory_relations(&[edge(a, b)], 0).unwrap();

    let embedding = SimulatedEmbeddingEndpoint::new(8, VectorNormalization::L2, 42);
    let profile = rel.establish_compatibility_profile(&embedding).unwrap();
    rel.build_missing_memory_vectors(profile.id, &embedding)
        .unwrap();
    let namer = DreamCommunityNamer::new(SimulatedGeneralEndpoint::new("dream-namer", vec![]));

    assert!(matches!(
        namer.name_communities(&mut rel, profile.id, 1),
        Err(DreamCommunityNamingError::MissingCurrentSnapshot)
    ));
}

#[test]
fn invalid_model_names_are_not_persisted() {
    let mut phy = Phylactery::create(path("dream-community-name-invalid.phy")).unwrap();
    let a = phy_memory(&mut phy, "a");
    let b = phy_memory(&mut phy, "b");
    phy.set_memory_relations(&[edge(a, b)], 0).unwrap();
    let embedding = SimulatedEmbeddingEndpoint::new(8, VectorNormalization::L2, 11);
    let profile = phy.establish_compatibility_profile(&embedding).unwrap();
    phy.build_missing_memory_vectors(profile.id, &embedding)
        .unwrap();
    phy.refresh_communities_leiden().unwrap();

    let namer = DreamCommunityNamer::new(SimulatedGeneralEndpoint::new(
        "dream-namer",
        vec![
            json!({"name": "   "}),
            json!({"name": "   "}),
            json!({"name": "   "}),
        ],
    ));
    assert!(matches!(
        namer.name_phylactery_communities(&mut phy, profile.id, 1),
        Err(DreamCommunityNamingError::InvalidOutput(_))
    ));
    assert!(phy.community_semantic_names().is_empty());
}
