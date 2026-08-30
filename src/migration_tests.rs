use crate::{
    Cva, FragmentConfig, GraphRelationKind, MemoryDraft, MigrationError, Phylactery,
    ReliquaryScopeKind, SimulatedEmbeddingEndpoint, VectorNormalization, migrate_file,
};
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

fn test_dir() -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("reliquary-migration-{unique}"));
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn draft(mutation: &str, memory_type: &str, content: &str) -> MemoryDraft {
    MemoryDraft {
        category: "fact".into(),
        memory_type: memory_type.into(),
        authority_kind: "direct".into(),
        title: mutation.into(),
        content: content.into(),
        scope: "private".into(),
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
        mutation_id: mutation.into(),
        created_at_ns: 1,
        updated_at_ns: 1,
    }
}

fn append_legacy_workspace_id(cva: &mut Cva, id: &str) {
    let mut payload = b"CVAWKSP1".to_vec();
    for value in [id, "Old Project", "construction"] {
        payload.extend_from_slice(&(value.len() as u32).to_le_bytes());
        payload.extend_from_slice(value.as_bytes());
    }
    cva.container.append(&payload).unwrap();
}

fn seed_archive(cva: &mut Cva) {
    for index in 0..8 {
        cva.append_node(
            format!("n{index}"),
            "c".into(),
            (index > 0).then(|| format!("n{}", index - 1)),
            if index % 2 == 0 { "user" } else { "assistant" }.into(),
            index,
            "migration archive text",
        )
        .unwrap();
    }
    cva.materialize_path_fragments(
        "c",
        "n7",
        FragmentConfig {
            turns: 4,
            overlap: 1,
        },
        true,
    )
    .unwrap();
}

#[test]
fn legacy_typed_rel_migration_preserves_identity_and_state() {
    let dir = test_dir();
    let source = dir.join("old.prj.rel");
    let copy = dir.join("old-copy.prj.rel");
    let output = dir.join("new.prj.rel");
    let copy_output = dir.join("new-copy.prj.rel");

    let mut rel = Cva::create_legacy_typed(&source, ReliquaryScopeKind::Project).unwrap();
    seed_archive(&mut rel);
    let (memory, _) = rel
        .publish_memory(None, 0, draft("project:m1", "project", "remember this"))
        .unwrap();
    let endpoint = SimulatedEmbeddingEndpoint::new(8, VectorNormalization::L2, 17);
    let profile = rel.establish_compatibility_profile(&endpoint).unwrap();
    rel.build_missing_memory_vectors(profile.id, &endpoint)
        .unwrap();
    rel.build_archive_vector_generation(profile.id, &endpoint)
        .unwrap();
    rel.put_conversation_compaction(
        "conversation-old".into(),
        "n5".into(),
        "durable migrated summary".into(),
        None,
    )
    .unwrap();
    append_legacy_workspace_id(&mut rel, "workspace-shared");
    rel.sync().unwrap();
    drop(rel);
    fs::copy(&source, &copy).unwrap();

    let first = migrate_file(&source, &output).unwrap();
    let second = migrate_file(&copy, &copy_output).unwrap();
    assert!(first.derived_from_legacy_workspace_id);
    assert_eq!(first.owner_id, second.owner_id);
    assert!(first.owner_id.starts_with("proj-"));

    let mut migrated = Cva::open(&output).unwrap();
    assert_eq!(migrated.stats().nodes, 8);
    assert_eq!(migrated.memory(memory.id).unwrap().content, "remember this");
    assert_eq!(migrated.compatibility_profile_stats().profiles, 1);
    assert_eq!(migrated.memory_vector_stats().bindings, 1);
    assert!(migrated.archive_vector_stats().objects > 0);
    assert_eq!(migrated.vector_generation_stats().generations, 1);
    assert!(migrated.current_vector_generation(profile.id).is_some());
    let compactions = migrated.conversation_compactions("conversation-old");
    assert_eq!(compactions.len(), 1);
    assert_eq!(compactions[0].through_message_id, "n5");
    assert_eq!(compactions[0].summary, "durable migrated summary");
}

#[test]
fn legacy_typed_phy_migration_preserves_owned_state() {
    let dir = test_dir();
    let source = dir.join("old.phy");
    let output = dir.join("new.phy");
    let mut phy = Phylactery::create_legacy_typed(&source).unwrap();
    let (a, _) = phy
        .publish_memory(None, 0, draft("user:a", "user", "prefers Helix"))
        .unwrap();
    let (b, _) = phy
        .publish_memory(None, 0, draft("user:b", "user", "prefers Nushell"))
        .unwrap();
    phy.set_memory_relation(a.id, b.id, GraphRelationKind::Topical, true, 0)
        .unwrap();
    let endpoint = SimulatedEmbeddingEndpoint::new(8, VectorNormalization::L2, 23);
    let profile = phy.establish_compatibility_profile(&endpoint).unwrap();
    phy.build_missing_memory_vectors(profile.id, &endpoint)
        .unwrap();
    phy.sync().unwrap();
    drop(phy);

    let result = migrate_file(&source, &output).unwrap();
    assert!(!result.derived_from_legacy_workspace_id);
    assert!(result.owner_id.starts_with("phy-"));

    let mut migrated = Phylactery::open(&output).unwrap();
    assert_eq!(migrated.memory(a.id).unwrap().content, "prefers Helix");
    assert_eq!(migrated.memory_stats().memories, 2);
    assert_eq!(migrated.graph_stats().active_relations, 1);
    assert_eq!(migrated.compatibility_profile_stats().profiles, 1);
    assert_eq!(migrated.memory_vector_stats().bindings, 2);
}

#[test]
fn legacy_cva_migrates_once_to_current_project_rel() {
    let dir = test_dir();
    let source = dir.join("old.cva");
    let output = dir.join("new.prj.rel");
    let retry_output = dir.join("retry.prj.rel");
    let mut legacy = Cva::create_legacy_cva(&source).unwrap();
    legacy
        .append_node("n".into(), "c".into(), None, "user".into(), 1, "legacy")
        .unwrap();
    legacy.sync().unwrap();
    drop(legacy);

    let result = migrate_file(&source, &output).unwrap();
    assert!(!result.derived_from_legacy_workspace_id);
    assert_eq!(result.scope, Some(ReliquaryScopeKind::Project));
    assert!(result.owner_id.starts_with("proj-"));
    let migrated = Cva::open(&output).unwrap();
    assert_eq!(migrated.stats().nodes, 1);
    assert!(migrated.conversation_compactions("c").is_empty());

    assert!(matches!(
        migrate_file(&output, &retry_output),
        Err(MigrationError::AlreadyCurrent)
    ));
    assert!(!retry_output.exists());
}
