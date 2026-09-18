use crate::{GeneralEndpoint, GeneralEndpointError, ReliquaryRuntimeHost};
use serde_json::{Value, json};
use std::sync::{Arc, Barrier};
use std::time::Duration;

pub(super) struct BlockingEntityEndpoint {
    pub(super) entered: Arc<Barrier>,
    pub(super) release: Arc<Barrier>,
}

impl GeneralEndpoint for BlockingEntityEndpoint {
    fn model(&self) -> &str {
        "runtime-host-blocking-entity-test"
    }

    fn complete_json(
        &self,
        _system_prompt: &str,
        _user_payload: &str,
        schema_name: &str,
        _schema: &Value,
    ) -> Result<Value, GeneralEndpointError> {
        if schema_name != "entity_admission_v1" {
            return Err(GeneralEndpointError::Failure(format!(
                "unexpected schema {schema_name}"
            )));
        }
        self.entered.wait();
        self.release.wait();
        Ok(json!({
            "decision": "create_new",
            "reason": "first_seen_identity",
            "entity_kind": "tool",
            "entity_summary": "Helix, a durable code editor."
        }))
    }
}

pub(super) fn wait_entities(host: &ReliquaryRuntimeHost, target: usize) {
    wait_until(
        || host.entity_stats().unwrap().entities >= target,
        "Reliquary Entity resolution",
    );
}

pub(super) fn wait_phylactery_entities(host: &ReliquaryRuntimeHost, target: usize) {
    wait_until(
        || {
            host.phylactery_entity_stats()
                .unwrap()
                .is_some_and(|stats| stats.entities >= target)
        },
        "Phylactery Entity resolution",
    );
}

pub(super) fn wait_phylactery_revisions(host: &ReliquaryRuntimeHost, target: usize) {
    wait_until(
        || {
            host.phylactery_memory_stats()
                .unwrap()
                .is_some_and(|stats| stats.revisions >= target)
        },
        "Phylactery Dream lifecycle",
    );
}

fn wait_until(mut ready: impl FnMut() -> bool, operation: &str) {
    for _ in 0..1_000 {
        if ready() {
            return;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    panic!("{operation} did not complete");
}
