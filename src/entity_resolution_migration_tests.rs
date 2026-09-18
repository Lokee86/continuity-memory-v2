use crate::{
    Cva, EntityDraft, EntityId, EntityResolutionReason, MemoryDraft, MemoryEntityMention,
    MemoryEntityMentionKey, MemoryEntityResolutionStatus, MemoryRoutingMetadata, MemoryTextField,
    Phylactery, ReliquaryScopeKind, migrate_file,
};

#[test]
fn rel_migration_preserves_entity_resolution_state() {
    let source = temp_path("old.prj.rel");
    let output = temp_path("new.prj.rel");
    let mut rel = Cva::create_legacy_typed(&source, ReliquaryScopeKind::Project).unwrap();
    let ids = publish_rel(&mut rel);
    let key = seed_memory(&mut rel.memories, &mut rel.container, ids);
    seed_rel_entity(&mut rel, EntityId([1; 32]), ids.0);
    rel.put_entity_unresolved(
        key,
        0,
        vec![EntityId([1; 32])],
        EntityResolutionReason::Ambiguous,
        [2; 32],
        [3; 32],
        10,
    )
    .unwrap();
    rel.sync().unwrap();
    drop(rel);

    migrate_file(&source, &output).unwrap();
    let migrated = Cva::open(&output).unwrap();
    assert!(matches!(
        migrated.entity_resolution(key).unwrap().status,
        MemoryEntityResolutionStatus::Pending(_)
    ));
}

#[test]
fn phy_migration_preserves_entity_resolution_state() {
    let source = temp_path("old.phy");
    let output = temp_path("new.phy");
    let mut phy = Phylactery::create_legacy_typed(&source).unwrap();
    let (memory, _) = phy.publish_memory(None, 0, draft()).unwrap();
    let body_id = phy.memory_body_id(memory.id).unwrap();
    let key = seed_memory_parts(&mut phy.memories, &mut phy.container, memory.id, body_id);
    seed_phy_entity(&mut phy, EntityId([9; 32]), memory.id);
    phy.put_entity_resolved(
        key,
        0,
        EntityId([9; 32]),
        EntityResolutionReason::ContextMatch,
        10,
    )
    .unwrap();
    phy.sync().unwrap();
    drop(phy);

    migrate_file(&source, &output).unwrap();
    let migrated = Phylactery::open(&output).unwrap();
    let MemoryEntityResolutionStatus::Resolved { entity_id, .. } =
        migrated.entity_resolution(key).unwrap().status
    else {
        panic!("expected resolved");
    };
    assert_eq!(entity_id, EntityId([9; 32]));
}

fn seed_rel_entity(rel: &mut Cva, id: EntityId, _memory_id: crate::MemoryId) {
    rel.publish_entity(Some(id), 0, entity_draft(id.0[0]))
        .unwrap();
}

fn seed_phy_entity(phy: &mut Phylactery, id: EntityId, _memory_id: crate::MemoryId) {
    phy.publish_entity(Some(id), 0, entity_draft(id.0[0]))
        .unwrap();
}

fn entity_draft(value: u8) -> EntityDraft {
    EntityDraft {
        canonical_name: format!("Migration Entity {value}"),
        aliases: vec![],
        kind: "test".into(),
        summary: "Migration Entity support.".into(),
        mutation_id: format!("entity-resolution-migration-entity-{value}"),
        created_at_ns: 1,
        updated_at_ns: 1,
    }
}

fn publish_rel(rel: &mut Cva) -> (crate::MemoryId, crate::MemoryBodyId) {
    let (memory, _) = rel.publish_memory(None, 0, draft()).unwrap();
    let body_id = rel.memory_body_id(memory.id).unwrap();
    (memory.id, body_id)
}

fn seed_memory(
    memories: &mut crate::memory_store::MemoryStore,
    container: &mut crate::Container,
    ids: (crate::MemoryId, crate::MemoryBodyId),
) -> MemoryEntityMentionKey {
    seed_memory_parts(memories, container, ids.0, ids.1)
}

fn seed_memory_parts(
    memories: &mut crate::memory_store::MemoryStore,
    container: &mut crate::Container,
    memory_id: crate::MemoryId,
    body_id: crate::MemoryBodyId,
) -> MemoryEntityMentionKey {
    let mention = MemoryEntityMention {
        field: MemoryTextField::Content,
        start_byte: 0,
        end_byte: 5,
        text: "Alpha".into(),
    };
    memories
        .put_routing_metadata(
            container,
            MemoryRoutingMetadata {
                memory_id,
                body_id,
                entity_mentions: vec![mention.clone()],
            },
        )
        .unwrap();
    MemoryEntityMentionKey::new(memory_id, &mention)
}

fn draft() -> MemoryDraft {
    MemoryDraft {
        category: "fact".into(),
        memory_type: "project".into(),
        authority_kind: "direct".into(),
        temporal_status: "current".into(),
        title: "Migration".into(),
        content: "Alpha survives migration.".into(),
        scope: "project".into(),
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
        mutation_id: format!("entity-resolution-migration-{}", uuid::Uuid::new_v4()),
        created_at_ns: 1,
        updated_at_ns: 1,
    }
}

fn temp_path(name: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!(
        "reliquary-entity-resolution-migration-{}-{name}",
        uuid::Uuid::new_v4()
    ))
}
