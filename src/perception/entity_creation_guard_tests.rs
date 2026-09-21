use crate::entity_candidate_test_support::{install_rel_mention, publish_rel_memory, temp_path};
use crate::{
    Cva, EntityCandidateConfig, EntityDraft, EntityError, EntityMaterialization,
    EntityResolutionDecision, EntityResolutionEvaluation, EntityResolutionPreparation,
    EntityResolutionReason, MemoryEntityResolutionStatus,
};

fn typed_entity_draft(name: &str, kind: &str, mutation: &str) -> EntityDraft {
    EntityDraft {
        canonical_name: name.into(),
        aliases: Vec::new(),
        kind: kind.into(),
        summary: format!("{name} durable identity"),
        mutation_id: mutation.into(),
        created_at_ns: 1,
        updated_at_ns: 1,
    }
}

fn commit_create(
    rel: &mut Cva,
    key: crate::MemoryEntityMentionKey,
    kind: &str,
    now_ns: i64,
) -> crate::EntityResolutionOutcome {
    let prepared = match rel
        .prepare_entity_resolution(key, EntityCandidateConfig::default())
        .unwrap()
    {
        EntityResolutionPreparation::Ready(value) => value,
        EntityResolutionPreparation::Complete(_) => panic!("mention unexpectedly terminal"),
    };
    rel.commit_entity_resolution(
        prepared,
        EntityResolutionEvaluation {
            output: crate::EntityResolverOutput {
                decision: EntityResolutionDecision::CreateNew,
                reason: EntityResolutionReason::ContextConflictNewIdentity,
            },
            materialization: Some(EntityMaterialization {
                kind: kind.into(),
                summary: format!("new {kind} identity"),
            }),
        },
        now_ns,
    )
    .unwrap()
    .unwrap()
}

#[test]
fn creation_guard_blocks_generic_suffix_duplicate() {
    let mut rel = Cva::create_project(temp_path("creation-guard-package.rel")).unwrap();
    let existing = rel
        .publish_entity(
            None,
            0,
            typed_entity_draft("entities package", "code_component", "entities-package"),
        )
        .unwrap()
        .0;
    let memory = publish_rel_memory(
        &mut rel,
        "entities-mention",
        "Entities",
        "The entities code owns the shared data models.",
    );
    let key = install_rel_mention(&mut rel, memory, "entities");

    let outcome = commit_create(&mut rel, key, "code_component", 10);

    assert_eq!(outcome.decision, EntityResolutionDecision::Unresolved);
    assert_eq!(outcome.reason, EntityResolutionReason::Ambiguous);
    assert!(!outcome.entity_created);
    assert_eq!(rel.entity_stats().entities, 1);
    let status = &rel.entity_resolution(key).unwrap().status;
    assert!(matches!(
        status,
        MemoryEntityResolutionStatus::Pending(value)
            if value.candidate_entity_ids.contains(&existing.id)
    ));
}

#[test]
fn creation_guard_blocks_path_and_language_qualified_duplicate() {
    let mut rel = Cva::create_project(temp_path("creation-guard-server.rel")).unwrap();
    let existing = rel
        .publish_entity(
            None,
            0,
            typed_entity_draft("services/game-server", "service", "game-server-service"),
        )
        .unwrap()
        .0;
    rel.publish_entity(
        None,
        0,
        typed_entity_draft("Go", "programming_language", "go-language"),
    )
    .unwrap();
    let memory = publish_rel_memory(
        &mut rel,
        "go-server-mention",
        "Server",
        "The Go Game Server owns live gameplay.",
    );
    let key = install_rel_mention(&mut rel, memory, "Go Game Server");

    let outcome = commit_create(&mut rel, key, "service", 11);

    assert_eq!(outcome.decision, EntityResolutionDecision::Unresolved);
    assert_eq!(rel.entity_stats().entities, 2);
    assert!(matches!(
        &rel.entity_resolution(key).unwrap().status,
        MemoryEntityResolutionStatus::Pending(value)
            if value.candidate_entity_ids.contains(&existing.id)
    ));
}

#[test]
fn creation_guard_allows_distinct_server_devtools_identity() {
    let mut rel = Cva::create_project(temp_path("creation-guard-devtools.rel")).unwrap();
    rel.publish_entity(
        None,
        0,
        typed_entity_draft("devtools", "subsystem", "client-devtools"),
    )
    .unwrap();
    let memory = publish_rel_memory(
        &mut rel,
        "server-devtools-mention",
        "Server tools",
        "Server devtools owns server-side debug commands.",
    );
    let key = install_rel_mention(&mut rel, memory, "Server devtools");

    let outcome = commit_create(&mut rel, key, "subsystem", 12);

    assert_eq!(outcome.decision, EntityResolutionDecision::CreateNew);
    assert!(outcome.entity_created);
    assert_eq!(rel.entity_stats().entities, 2);
}

#[test]
fn context_only_resolution_does_not_promote_global_alias() {
    let mut rel = Cva::create_project(temp_path("context-alias-guard.rel")).unwrap();
    let entity = rel
        .publish_entity(
            None,
            0,
            typed_entity_draft("Lightweight Local Server", "service", "local-server"),
        )
        .unwrap()
        .0;
    let memory = publish_rel_memory(
        &mut rel,
        "generic-server-mention",
        "Local play",
        "The game server handles the local single-player session.",
    );
    let key = install_rel_mention(&mut rel, memory, "game server");
    rel.set_entity_association(memory, entity.id, true, rel.graph_version())
        .unwrap();
    let prepared = match rel
        .prepare_entity_resolution(key, EntityCandidateConfig::default())
        .unwrap()
    {
        EntityResolutionPreparation::Ready(value) => value,
        EntityResolutionPreparation::Complete(_) => panic!("mention unexpectedly terminal"),
    };
    let candidate = prepared
        .candidates
        .candidates
        .iter()
        .find(|candidate| candidate.entity.id == entity.id)
        .unwrap();
    assert!(!candidate.exact_surface);
    assert!(!candidate.normalized_surface);
    assert!(!candidate.alias_surface);

    rel.commit_entity_resolution(
        prepared,
        EntityResolutionEvaluation {
            output: crate::EntityResolverOutput {
                decision: EntityResolutionDecision::ResolveExisting(entity.id),
                reason: EntityResolutionReason::ContextMatch,
            },
            materialization: None,
        },
        13,
    )
    .unwrap()
    .unwrap();

    assert!(
        !rel.entity(entity.id)
            .unwrap()
            .aliases
            .iter()
            .any(|alias| alias.eq_ignore_ascii_case("game server"))
    );
}

#[test]
fn merge_retargets_state_graph_and_survives_reopen() {
    let path = temp_path("entity-merge.rel");
    let mut rel = Cva::create_project(&path).unwrap();
    let survivor = rel
        .publish_entity(
            None,
            0,
            typed_entity_draft("game server", "service", "game-server-canonical"),
        )
        .unwrap()
        .0;
    let retired = rel
        .publish_entity(
            None,
            0,
            typed_entity_draft("services/game-server", "service", "game-server-path"),
        )
        .unwrap()
        .0;
    let memory = publish_rel_memory(
        &mut rel,
        "merge-memory",
        "Server",
        "services/game-server owns live gameplay.",
    );
    let key = install_rel_mention(&mut rel, memory, "services/game-server");
    rel.set_entity_association(memory, retired.id, true, rel.graph_version())
        .unwrap();
    rel.put_entity_resolved(key, 0, retired.id, EntityResolutionReason::ContextMatch, 20)
        .unwrap();

    let outcome = rel.merge_entities(survivor.id, retired.id, 21).unwrap();

    assert!(outcome.changed);
    assert_eq!(outcome.associations_retargeted, 1);
    assert_eq!(outcome.resolutions_retargeted, 1);
    assert!(matches!(
        rel.entity(retired.id),
        Err(EntityError::MissingEntity)
    ));
    assert_eq!(
        rel.retired_entity_replacement(retired.id),
        Some(survivor.id)
    );
    assert_eq!(
        rel.entity_associations_for_memory(memory),
        vec![survivor.id]
    );
    assert!(matches!(
        rel.entity_resolution(key).unwrap().status,
        MemoryEntityResolutionStatus::Resolved { entity_id, .. } if entity_id == survivor.id
    ));
    assert!(
        rel.entity(survivor.id)
            .unwrap()
            .aliases
            .iter()
            .any(|alias| alias == "services/game-server")
    );
    rel.sync().unwrap();
    drop(rel);

    let mut reopened = Cva::open(&path).unwrap();
    assert!(matches!(
        reopened.entity(retired.id),
        Err(EntityError::MissingEntity)
    ));
    assert_eq!(
        reopened.retired_entity_replacement(retired.id),
        Some(survivor.id)
    );
    assert_eq!(
        reopened.entity_associations_for_memory(memory),
        vec![survivor.id]
    );
    let second = reopened
        .merge_entities(survivor.id, retired.id, 22)
        .unwrap();
    assert!(!second.changed);
}
