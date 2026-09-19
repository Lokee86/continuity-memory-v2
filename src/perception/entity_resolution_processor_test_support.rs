use crate::{GeneralEndpoint, GeneralEndpointError};
use serde_json::{Value, json};
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

pub(crate) struct BootstrapEndpoint {
    calls: Arc<AtomicUsize>,
}

impl BootstrapEndpoint {
    pub(crate) fn new() -> (Self, Arc<AtomicUsize>) {
        let calls = Arc::new(AtomicUsize::new(0));
        (
            Self {
                calls: calls.clone(),
            },
            calls,
        )
    }
}

impl GeneralEndpoint for BootstrapEndpoint {
    fn model(&self) -> &str {
        "bootstrap-test"
    }

    fn complete_json(
        &self,
        _system_prompt: &str,
        user_payload: &str,
        schema_name: &str,
        _schema: &Value,
    ) -> Result<Value, GeneralEndpointError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        let payload: Value = serde_json::from_str(user_payload)
            .map_err(|error| GeneralEndpointError::Failure(error.to_string()))?;
        let content = payload["content"].as_str().unwrap_or_default();

        if schema_name == "entity_admission_v1" {
            if content.contains("not a durable referent") {
                return Ok(admission("reject", "generic_role", "unknown", ""));
            }
            if content.contains("ambiguous executable") {
                return Ok(admission("unresolved", "ambiguous", "unknown", ""));
            }
            let (kind, summary) = if content.contains("telemetry ingestion service") {
                (
                    "service",
                    "The Helix telemetry ingestion service for application events.",
                )
            } else {
                ("tool", "The Helix code editor used for Rust work.")
            };
            return Ok(admission(
                "create_new",
                "first_seen_identity",
                kind,
                summary,
            ));
        }

        if schema_name == "entity_materialization_v1" {
            return Ok(json!({
                "entity_kind": "service",
                "entity_summary": "The Helix telemetry ingestion service for application events."
            }));
        }

        let candidates = payload["candidates"]
            .as_array()
            .cloned()
            .unwrap_or_default();
        if content.contains("telemetry ingestion service") {
            return Ok(decision("create_new", "context_conflict_new_identity", -1));
        }
        if content.contains("telemetry ingestion service") {
            return Ok(decision("create_new", "context_conflict_new_identity", -1));
        }

        let wanted = if content.contains("telemetry queue") {
            "telemetry ingestion service"
        } else if content.contains("keybindings") || content.contains("editor restart") {
            "code editor"
        } else {
            ""
        };
        if !wanted.is_empty() {
            if let Some(index) = candidates.iter().position(|candidate| {
                candidate["evidence"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .any(|memory| {
                        memory["text"]
                            .as_str()
                            .is_some_and(|text| text.contains(wanted))
                    })
            }) {
                return Ok(decision("resolve_existing", "context_match", index as i64));
            }
        }

        Ok(decision("unresolved", "insufficient_evidence", -1))
    }
}

fn decision(decision: &str, reason: &str, target: i64) -> Value {
    json!({
        "decision": decision,
        "reason": reason,
        "target_candidate_index": target,
    })
}

fn admission(decision: &str, reason: &str, kind: &str, summary: &str) -> Value {
    json!({
        "decision": decision,
        "reason": reason,
        "entity_kind": kind,
        "entity_summary": summary,
    })
}
