use crate::entity_candidate_test_support::{install_rel_mention, publish_rel_memory, temp_path};
use crate::{
    Cva, EntityCandidateConfig, EntityResolutionEngine, GeneralEndpoint, GeneralEndpointError,
};
use serde_json::{Value, json};
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};
use std::time::Duration;

struct RetryOnceEndpoint {
    calls: Arc<AtomicUsize>,
}

impl GeneralEndpoint for RetryOnceEndpoint {
    fn model(&self) -> &str {
        "parallel-retry-test"
    }

    fn complete_json(
        &self,
        _system_prompt: &str,
        _user_payload: &str,
        schema_name: &str,
        _schema: &Value,
    ) -> Result<Value, GeneralEndpointError> {
        assert_eq!(schema_name, "entity_admission_v6");
        if self.calls.fetch_add(1, Ordering::SeqCst) == 0 {
            return Err(GeneralEndpointError::backpressure(
                "simulated transient backpressure",
                Some(Duration::ZERO),
            ));
        }
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
fn parallel_batch_retries_transient_endpoint_failure() {
    let mut rel = Cva::create_project(temp_path("parallel-retry.rel")).unwrap();
    let memory = publish_rel_memory(
        &mut rel,
        "retry",
        "Retry",
        "RetryTool is a durable tool with a distinct identity.",
    );
    let key = install_rel_mention(&mut rel, memory, "RetryTool");
    let calls = Arc::new(AtomicUsize::new(0));
    let engine = EntityResolutionEngine::new(RetryOnceEndpoint {
        calls: Arc::clone(&calls),
    });
    let config = EntityCandidateConfig {
        lexical_memory_limit: 0,
        graph_neighbor_limit: 0,
        ..EntityCandidateConfig::default()
    };

    let batch = rel
        .resolve_entity_mentions_with_engine_parallel(&engine, &[key], config, 500, 1)
        .unwrap();

    assert_eq!(batch.outcomes.len(), 1);
    assert_eq!(batch.speculative_evaluations, 1);
    assert_eq!(calls.load(Ordering::SeqCst), 2);
    assert_eq!(rel.entities().len(), 1);
    assert_eq!(rel.entity_resolutions().len(), 1);
}
