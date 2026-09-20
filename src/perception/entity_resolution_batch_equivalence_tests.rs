use crate::entity_candidate_test_support::{install_rel_mention, publish_rel_memory, temp_path};
use crate::entity_resolution_processor_test_support::BootstrapEndpoint;
use crate::{Cva, EntityCandidateConfig, EntityResolutionEngine, MemoryEntityResolutionStatus};

fn install_helix_case(rel: &mut Cva) -> [crate::MemoryEntityMentionKey; 4] {
    let editor_first = publish_rel_memory(
        rel,
        "editor-first",
        "Editor",
        "Helix is the code editor installed for Rust work.",
    );
    let editor_again = publish_rel_memory(
        rel,
        "editor-again",
        "Editor settings",
        "Helix keybindings should map save to Ctrl-S.",
    );
    let service_first = publish_rel_memory(
        rel,
        "service-first",
        "Telemetry",
        "Helix is the telemetry ingestion service for application events.",
    );
    let service_again = publish_rel_memory(
        rel,
        "service-again",
        "Telemetry queue",
        "Helix telemetry queue latency increased after deployment.",
    );
    [
        install_rel_mention(rel, editor_first, "Helix"),
        install_rel_mention(rel, editor_again, "Helix"),
        install_rel_mention(rel, service_first, "Helix"),
        install_rel_mention(rel, service_again, "Helix"),
    ]
}

#[test]
fn parallel_state_evolution_matches_serial_identity_structure() {
    let mut serial = Cva::create_project(temp_path("batch-equivalence-serial.rel")).unwrap();
    let serial_keys = install_helix_case(&mut serial);
    let (serial_endpoint, _) = BootstrapEndpoint::new();
    let serial_engine = EntityResolutionEngine::new(serial_endpoint);
    let config = EntityCandidateConfig::default();
    let mut serial_created = Vec::new();
    for (index, key) in serial_keys.iter().enumerate() {
        let outcome = serial
            .resolve_entity_mention_with_engine(&serial_engine, *key, config, 100 + index as i64)
            .unwrap();
        serial_created.push(outcome.entity_created);
    }

    let mut parallel = Cva::create_project(temp_path("batch-equivalence-parallel.rel")).unwrap();
    let parallel_keys = install_helix_case(&mut parallel);
    let (parallel_endpoint, _) = BootstrapEndpoint::new();
    let parallel_engine = EntityResolutionEngine::new(parallel_endpoint);
    let batch = parallel
        .resolve_entity_mentions_with_engine_parallel(
            &parallel_engine,
            &parallel_keys,
            config,
            100,
            4,
        )
        .unwrap();

    assert_eq!(
        batch
            .outcomes
            .iter()
            .map(|outcome| outcome.entity_created)
            .collect::<Vec<_>>(),
        serial_created
    );
    assert_eq!(parallel.entities().len(), serial.entities().len());
    let mut serial_kinds = serial
        .entities()
        .into_iter()
        .map(|entity| entity.kind)
        .collect::<Vec<_>>();
    let mut parallel_kinds = parallel
        .entities()
        .into_iter()
        .map(|entity| entity.kind)
        .collect::<Vec<_>>();
    serial_kinds.sort();
    parallel_kinds.sort();
    assert_eq!(parallel_kinds, serial_kinds);
    assert_eq!(parallel_kinds, vec!["service", "tool"]);

    for key in parallel_keys {
        assert!(matches!(
            parallel.entity_resolution(key).unwrap().status,
            MemoryEntityResolutionStatus::Resolved { .. }
        ));
    }
    assert!(batch.reevaluations >= 2);
}
