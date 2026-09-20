use crate::entity_candidate_test_support::{install_rel_mention, publish_rel_memory, temp_path};
use crate::{
    Cva, EntityCandidateConfig, EntityResolutionEngine, GeneralEndpoint, GeneralEndpointError,
};
use serde_json::{Value, json};

struct FailingWaveEndpoint;

impl GeneralEndpoint for FailingWaveEndpoint {
    fn model(&self) -> &str {
        "failing-wave-test"
    }

    fn complete_json(
        &self,
        _system_prompt: &str,
        user_payload: &str,
        schema_name: &str,
        _schema: &Value,
    ) -> Result<Value, GeneralEndpointError> {
        assert_eq!(schema_name, "entity_admission_v6");
        if user_payload.contains("FailTool") {
            return Err(GeneralEndpointError::Failure(
                "simulated endpoint failure".into(),
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
fn failed_parallel_wave_commits_nothing_from_that_wave() {
    let mut rel = Cva::create_project(temp_path("parallel-failure-atomic.rel")).unwrap();
    let good_memory = publish_rel_memory(
        &mut rel,
        "good",
        "Good",
        "GoodTool is a durable tool with a distinct identity.",
    );
    let fail_memory = publish_rel_memory(
        &mut rel,
        "fail",
        "Fail",
        "FailTool is a durable tool with a distinct identity.",
    );
    let keys = [
        install_rel_mention(&mut rel, good_memory, "GoodTool"),
        install_rel_mention(&mut rel, fail_memory, "FailTool"),
    ];
    let config = EntityCandidateConfig {
        lexical_memory_limit: 0,
        graph_neighbor_limit: 0,
        ..EntityCandidateConfig::default()
    };

    let engine = EntityResolutionEngine::new(FailingWaveEndpoint);
    let result = rel.resolve_entity_mentions_with_engine_parallel(&engine, &keys, config, 400, 2);

    assert!(result.is_err());
    assert!(rel.entities().is_empty());
    assert!(rel.entity_resolutions().is_empty());
}
