use crate::{
    Container, Cva, CvaError, GraphRelationKind, MemoryDraft, MemoryError, Phylactery,
    PhylacteryError, SimulatedEmbeddingEndpoint, VectorNormalization,
};
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

fn test_path(name: &str) -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("reliquary-phylactery-{unique}"));
    fs::create_dir_all(&dir).unwrap();
    dir.join(name)
}

fn draft(mutation_id: &str, title: &str, content: &str) -> MemoryDraft {
    MemoryDraft {
        category: "preference".into(),
        memory_type: "user".into(),
        authority_kind: "direct".into(),
        title: title.into(),
        content: content.into(),
        scope: "user".into(),
        lifecycle_state: "extracted".into(),
        archived: false,
        superseded_by: None,
        parent_id: None,
        source_node_id: None,
        content_source_conversation_id: None,
        content_source_node_id: None,
        grounding_source_conversation_id: None,
        grounding_source_node_id: None,
        source_episode_id: None,
        source_time_ns: None,
        mutation_id: mutation_id.into(),
        created_at_ns: 10,
        updated_at_ns: 10,
    }
}

#[test]
fn phylactery_identity_and_source_independent_memory_reopen() {
    let path = test_path("user.phy");
    let mut phy = Phylactery::create(&path).unwrap();
    assert_eq!(
        phy.container.identity().unwrap().file_kind,
        crate::FileKind::Phylactery
    );
    let (memory, created) = phy
        .publish_memory(
            None,
            0,
            draft("user:m1", "Editor", "The user prefers Helix."),
        )
        .unwrap();
    assert!(created);
    phy.sync().unwrap();
    drop(phy);

    let mut reopened = Phylactery::open(&path).unwrap();
    assert_eq!(reopened.memory_stats().memories, 1);
    assert_eq!(reopened.memory(memory.id).unwrap().content, memory.content);
}

#[test]
fn phylactery_rejects_rel_local_provenance() {
    let path = test_path("provenance.phy");
    let mut phy = Phylactery::create(path).unwrap();
    let mut sourced = draft("user:sourced", "Preference", "Keep this.");
    sourced.source_node_id = Some("project-node".into());
    assert!(matches!(
        phy.publish_memory(None, 0, sourced),
        Err(MemoryError::InvalidProvenance)
    ));
}

#[test]
fn reliquary_and_phylactery_reject_each_others_file_kind() {
    let phy_path = test_path("wrong.phy");
    drop(Phylactery::create(&phy_path).unwrap());
    assert!(matches!(
        Cva::open(&phy_path),
        Err(CvaError::InvalidContainerIdentity(_))
    ));

    let rel_path = test_path("wrong.prj.rel");
    drop(Cva::create_project(&rel_path).unwrap());
    assert!(matches!(
        Phylactery::open(&rel_path),
        Err(PhylacteryError::InvalidContainerIdentity(_))
    ));

    let legacy_path = test_path("legacy.cva");
    drop(Container::create(&legacy_path).unwrap());
    assert!(matches!(
        Phylactery::open(&legacy_path),
        Err(PhylacteryError::InvalidContainerIdentity(_))
    ));
}

#[test]
fn phylactery_rejects_rel_only_owner_records() {
    let path = test_path("foreign-owner.phy");
    let mut phy = Phylactery::create(&path).unwrap();
    phy.container.append(b"CVAAFMT2foreign").unwrap();
    phy.sync().unwrap();
    drop(phy);

    assert!(matches!(
        Phylactery::open(&path),
        Err(PhylacteryError::InvalidContainerIdentity(_))
    ));
}

#[test]
fn graph_profiles_and_memory_vectors_persist() {
    let path = test_path("state.phy");
    let mut phy = Phylactery::create(&path).unwrap();
    let (a, _) = phy
        .publish_memory(
            None,
            0,
            draft("user:a", "Editor", "The user prefers Helix."),
        )
        .unwrap();
    let (b, _) = phy
        .publish_memory(
            None,
            0,
            draft("user:b", "Shell", "The user prefers Nushell."),
        )
        .unwrap();
    phy.set_memory_relation(a.id, b.id, GraphRelationKind::Topical, true, 0)
        .unwrap();

    let endpoint = SimulatedEmbeddingEndpoint::new(8, VectorNormalization::L2, 7);
    let profile = phy.establish_compatibility_profile(&endpoint).unwrap();
    let result = phy
        .build_missing_memory_vectors(profile.id, &endpoint)
        .unwrap();
    assert_eq!(result.embedded, 2);
    phy.sync().unwrap();
    drop(phy);

    let reopened = Phylactery::open(&path).unwrap();
    assert_eq!(reopened.graph_stats().active_relations, 1);
    assert_eq!(reopened.compatibility_profile_stats().profiles, 1);
    assert_eq!(reopened.memory_vector_stats().bindings, 2);
}
