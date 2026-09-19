use crate::entity_candidate_test_support::{
    entity_draft, install_phy_mention, install_rel_mention, publish_phy_memory, publish_rel_memory,
    temp_path,
};
use crate::entity_resolution_processor_test_support::BootstrapEndpoint;
use crate::{
    Cva, EntityCandidateConfig, EntityResolutionDecision, EntityResolutionReason, EntityResolver,
    MemoryEntityResolutionStatus, Phylactery, SimulatedGeneralEndpoint,
};
use serde_json::json;
use std::sync::atomic::Ordering;

#[test]
fn zero_entity_bootstrap_creates_splits_and_converges_after_reopen() {
    let path = temp_path("bootstrap.rel");
    let mut rel = Cva::create_project(&path).unwrap();

    let editor_first = publish_rel_memory(
        &mut rel,
        "editor-first",
        "Editor",
        "Helix is the code editor installed for Rust work.",
    );
    let editor_again = publish_rel_memory(
        &mut rel,
        "editor-again",
        "Editor settings",
        "Helix keybindings should map save to Ctrl-S.",
    );
    let service_first = publish_rel_memory(
        &mut rel,
        "service-first",
        "Telemetry",
        "Helix is the telemetry ingestion service for application events.",
    );
    let service_again = publish_rel_memory(
        &mut rel,
        "service-again",
        "Telemetry queue",
        "Helix telemetry queue latency increased after deployment.",
    );

    let keys = [
        install_rel_mention(&mut rel, editor_first, "Helix"),
        install_rel_mention(&mut rel, editor_again, "Helix"),
        install_rel_mention(&mut rel, service_first, "Helix"),
        install_rel_mention(&mut rel, service_again, "Helix"),
    ];

    let (endpoint, calls) = BootstrapEndpoint::new();
    let resolver = EntityResolver::new(endpoint);
    let config = EntityCandidateConfig::default();

    let first = rel
        .resolve_entity_mention(&resolver, keys[0], config, 10)
        .unwrap();
    assert!(first.entity_created);
    let editor = first.entity_id.unwrap();
    assert_eq!(rel.entities().len(), 1);
    assert_eq!(
        rel.entity_associations_for_memory(editor_first),
        vec![editor]
    );

    let second = rel
        .resolve_entity_mention(&resolver, keys[1], config, 11)
        .unwrap();
    assert_eq!(second.entity_id, Some(editor));
    assert!(!second.entity_created);
    assert_eq!(rel.entities().len(), 1);

    let third = rel
        .resolve_entity_mention(&resolver, keys[2], config, 12)
        .unwrap();
    let service = third.entity_id.unwrap();
    assert!(third.entity_created);
    assert_ne!(service, editor);
    assert_eq!(rel.entities().len(), 2);

    let fourth = rel
        .resolve_entity_mention(&resolver, keys[3], config, 13)
        .unwrap();
    assert_eq!(fourth.entity_id, Some(service));
    assert_eq!(rel.entities().len(), 2);
    assert_eq!(calls.load(Ordering::SeqCst), 5);

    for (key, expected) in keys.into_iter().zip([editor, editor, service, service]) {
        assert!(matches!(
            rel.entity_resolution(key).unwrap().status,
            MemoryEntityResolutionStatus::Resolved { entity_id, .. } if entity_id == expected
        ));
    }

    rel.sync().unwrap();
    drop(rel);
    let reopened = Cva::open(path).unwrap();
    assert_eq!(reopened.entities().len(), 2);
    assert_eq!(
        reopened.entity_associations_for_memory(editor_first),
        vec![editor]
    );
    assert_eq!(
        reopened.entity_associations_for_memory(service_first),
        vec![service]
    );
}

#[test]
fn unresolved_and_rejected_states_are_persisted_and_do_not_repeat_inference() {
    let mut rel = Cva::create_project(temp_path("terminal-and-pending.rel")).unwrap();
    let ambiguous = publish_rel_memory(
        &mut rel,
        "ambiguous",
        "Entrypoint",
        "main.go is an ambiguous executable entry point.",
    );
    let rejected = publish_rel_memory(
        &mut rel,
        "rejected",
        "Role",
        "Operator is not a durable referent in this sentence.",
    );
    let ambiguous_key = install_rel_mention(&mut rel, ambiguous, "main.go");
    let rejected_key = install_rel_mention(&mut rel, rejected, "Operator");

    let (endpoint, calls) = BootstrapEndpoint::new();
    let resolver = EntityResolver::new(endpoint);
    let config = EntityCandidateConfig::default();

    let pending = rel
        .resolve_entity_mention(&resolver, ambiguous_key, config, 20)
        .unwrap();
    assert_eq!(pending.decision, EntityResolutionDecision::Unresolved);
    assert!(pending.resolution_changed);
    assert!(matches!(
        rel.entity_resolution(ambiguous_key).unwrap().status,
        MemoryEntityResolutionStatus::Pending(_)
    ));

    let repeated = rel
        .resolve_entity_mention(&resolver, ambiguous_key, config, 21)
        .unwrap();
    assert_eq!(repeated.decision, EntityResolutionDecision::Unresolved);
    assert!(!repeated.resolution_changed);

    let rejected_outcome = rel
        .resolve_entity_mention(&resolver, rejected_key, config, 22)
        .unwrap();
    assert_eq!(rejected_outcome.decision, EntityResolutionDecision::Reject);
    assert!(matches!(
        rel.entity_resolution(rejected_key).unwrap().status,
        MemoryEntityResolutionStatus::Rejected {
            reason: EntityResolutionReason::GenericRole
        }
    ));

    let repeated_reject = rel
        .resolve_entity_mention(&resolver, rejected_key, config, 23)
        .unwrap();
    assert!(!repeated_reject.resolution_changed);
    assert_eq!(calls.load(Ordering::SeqCst), 2);
}

#[test]
fn existing_source_association_recovers_partial_resolution_write() {
    let mut rel = Cva::create_project(temp_path("association-recovery.rel")).unwrap();
    let memory = publish_rel_memory(
        &mut rel,
        "recovery",
        "Editor restart",
        "Zephyr editor restart should restore the previous workspace.",
    );
    let key = install_rel_mention(&mut rel, memory, "Zephyr");

    let entity_id = rel
        .publish_entity(
            None,
            0,
            entity_draft("Editor Product", &[], "partial-entity", 1),
        )
        .unwrap()
        .0
        .id;
    rel.set_entity_association(memory, entity_id, true, rel.graph_version())
        .unwrap();

    let endpoint = SimulatedGeneralEndpoint::new(
        "resolver-test",
        vec![json!({
            "decision": "resolve_existing",
            "reason": "context_match",
            "target_candidate_index": 0
        })],
    );
    let resolver = EntityResolver::new(endpoint);
    let result = rel
        .resolve_entity_mention(&resolver, key, EntityCandidateConfig::default(), 30)
        .unwrap();

    assert_eq!(result.entity_id, Some(entity_id));
    assert!(!result.association_changed);
    assert!(result.resolution_changed);
}

#[test]
fn phylactery_uses_same_zero_entity_create_path() {
    let mut phy = Phylactery::create(temp_path("bootstrap.phy")).unwrap();
    let memory = publish_phy_memory(&mut phy, "Zephyr is a durable local editor.");
    let key = install_phy_mention(&mut phy, memory, "Zephyr");
    let resolver = EntityResolver::new(SimulatedGeneralEndpoint::new(
        "resolver-test",
        vec![json!({
            "decision": "create_new",
            "reason": "first_seen_identity",
            "entity_kind": "tool",
            "entity_summary": "The Zephyr local editor."
        })],
    ));

    let result = phy
        .resolve_entity_mention(&resolver, key, EntityCandidateConfig::default(), 40)
        .unwrap();

    let entity_id = result.entity_id.unwrap();
    assert!(result.entity_created);
    assert_eq!(phy.entities().len(), 1);
    assert_eq!(phy.entity_associations_for_memory(memory), vec![entity_id]);
    assert!(matches!(
        phy.entity_resolution(key).unwrap().status,
        MemoryEntityResolutionStatus::Resolved { entity_id: id, .. } if id == entity_id
    ));
}
