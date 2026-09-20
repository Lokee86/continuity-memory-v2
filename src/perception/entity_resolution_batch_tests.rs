use crate::entity_candidate_test_support::{install_rel_mention, publish_rel_memory, temp_path};
use crate::entity_resolution_processor_test_support::BootstrapEndpoint;
use crate::{
    Cva, EntityCandidateConfig, EntityResolutionDecision, EntityResolutionEngine, GeneralEndpoint,
    GeneralEndpointError, MemoryEntityResolutionStatus,
};
use serde_json::{Value, json};
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};
use std::time::Duration;

#[test]
fn parallel_wave_revalidates_stale_same_surface_before_commit() {
    let mut rel = Cva::create_project(temp_path("parallel-revalidate.rel")).unwrap();
    let first_memory = publish_rel_memory(
        &mut rel,
        "editor-first",
        "Editor",
        "Helix is the code editor installed for Rust work.",
    );
    let second_memory = publish_rel_memory(
        &mut rel,
        "editor-again",
        "Editor settings",
        "Helix keybindings should map save to Ctrl-S.",
    );
    let keys = [
        install_rel_mention(&mut rel, first_memory, "Helix"),
        install_rel_mention(&mut rel, second_memory, "Helix"),
    ];

    let (endpoint, calls) = BootstrapEndpoint::new();
    let engine = EntityResolutionEngine::new(endpoint);
    let batch = rel
        .resolve_entity_mentions_with_engine_parallel(
            &engine,
            &keys,
            EntityCandidateConfig::default(),
            100,
            2,
        )
        .unwrap();

    assert_eq!(batch.speculative_evaluations, 2);
    assert_eq!(batch.reevaluations, 1);
    assert_eq!(batch.outcomes.len(), 2);
    assert_eq!(rel.entities().len(), 1);
    let entity_id = batch.outcomes[0].entity_id.unwrap();
    assert_eq!(batch.outcomes[1].entity_id, Some(entity_id));
    assert_eq!(
        batch.outcomes[1].decision,
        EntityResolutionDecision::ResolveExisting(entity_id)
    );
    assert_eq!(calls.load(Ordering::SeqCst), 3);
    for key in keys {
        assert!(matches!(
            rel.entity_resolution(key).unwrap().status,
            MemoryEntityResolutionStatus::Resolved { entity_id: id, .. } if id == entity_id
        ));
    }
}

#[test]
fn parallel_wave_reuses_evaluations_when_relevant_snapshot_is_unchanged() {
    let mut rel = Cva::create_project(temp_path("parallel-reuse.rel")).unwrap();
    let alpha_memory = publish_rel_memory(
        &mut rel,
        "alpha",
        "Alpha",
        "AlphaTool is the durable editor used for alpha work.",
    );
    let beta_memory = publish_rel_memory(
        &mut rel,
        "beta",
        "Beta",
        "BetaTool is the durable editor used for beta work.",
    );
    let keys = [
        install_rel_mention(&mut rel, alpha_memory, "AlphaTool"),
        install_rel_mention(&mut rel, beta_memory, "BetaTool"),
    ];
    let config = EntityCandidateConfig {
        lexical_memory_limit: 0,
        graph_neighbor_limit: 0,
        ..EntityCandidateConfig::default()
    };

    let (endpoint, calls) = BootstrapEndpoint::new();
    let engine = EntityResolutionEngine::new(endpoint);
    let batch = rel
        .resolve_entity_mentions_with_engine_parallel(&engine, &keys, config, 200, 2)
        .unwrap();

    assert_eq!(batch.speculative_evaluations, 2);
    assert_eq!(batch.reevaluations, 0);
    assert_eq!(rel.entities().len(), 2);
    assert_eq!(calls.load(Ordering::SeqCst), 2);
}

struct ConcurrentAdmissionEndpoint {
    active: Arc<AtomicUsize>,
    max_active: Arc<AtomicUsize>,
}

impl GeneralEndpoint for ConcurrentAdmissionEndpoint {
    fn model(&self) -> &str {
        "concurrency-test"
    }

    fn complete_json(
        &self,
        _system_prompt: &str,
        _user_payload: &str,
        schema_name: &str,
        _schema: &Value,
    ) -> Result<Value, GeneralEndpointError> {
        assert_eq!(schema_name, "entity_admission_v6");
        let active = self.active.fetch_add(1, Ordering::SeqCst) + 1;
        self.max_active.fetch_max(active, Ordering::SeqCst);
        std::thread::sleep(Duration::from_millis(40));
        self.active.fetch_sub(1, Ordering::SeqCst);
        Ok(json!({
            "decision": "create_new",
            "reason": "first_seen_identity",
            "promotion_policy": "immediate",
            "entity_kind": "tool",
            "entity_summary": "A durable test tool."
        }))
    }
}

#[test]
fn parallel_wave_overlaps_endpoint_calls() {
    let mut rel = Cva::create_project(temp_path("parallel-overlap.rel")).unwrap();
    let mut keys = Vec::new();
    for name in ["AlphaTool", "BetaTool", "GammaTool", "DeltaTool"] {
        let memory = publish_rel_memory(
            &mut rel,
            name,
            name,
            &format!("{name} is a durable tool with a distinct identity."),
        );
        keys.push(install_rel_mention(&mut rel, memory, name));
    }
    let active = Arc::new(AtomicUsize::new(0));
    let max_active = Arc::new(AtomicUsize::new(0));
    let engine = EntityResolutionEngine::new(ConcurrentAdmissionEndpoint {
        active,
        max_active: Arc::clone(&max_active),
    });
    let config = EntityCandidateConfig {
        lexical_memory_limit: 0,
        graph_neighbor_limit: 0,
        ..EntityCandidateConfig::default()
    };

    let batch = rel
        .resolve_entity_mentions_with_engine_parallel(&engine, &keys, config, 300, 4)
        .unwrap();

    assert_eq!(batch.speculative_evaluations, 4);
    assert_eq!(batch.reevaluations, 0);
    assert_eq!(rel.entities().len(), 4);
    assert!(
        max_active.load(Ordering::SeqCst) >= 2,
        "endpoint calls did not overlap"
    );
}
