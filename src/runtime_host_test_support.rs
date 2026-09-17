use crate::{
    Cva, EmbeddingEndpoint, EmbeddingEndpointError, EmbeddingMode, EpisodeConfig, GeneralEndpoint,
    GeneralEndpointError, InsomniaWorkerConfig, ReliquaryRuntimeHost, SimulatedEmbeddingEndpoint,
    VectorNormalization,
};
use serde_json::{Value, json};
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Barrier};
use std::time::Duration;

pub(super) fn test_path(name: &str) -> PathBuf {
    let unique = uuid::Uuid::new_v4();
    let dir = std::env::temp_dir().join(format!("reliquary-runtime-host-{unique}"));
    fs::create_dir_all(&dir).unwrap();
    dir.join(name)
}

fn omit_ledger(payload: &str) -> Result<Value, GeneralEndpointError> {
    let payload: Value = serde_json::from_str(payload)
        .map_err(|error| GeneralEndpointError::Failure(error.to_string()))?;
    let mut turns = serde_json::Map::new();
    for turn in payload["turns"].as_array().into_iter().flatten() {
        if turn["role"] != "user" {
            continue;
        }
        let id = turn["id"].as_str().unwrap_or("");
        turns.insert(
            id.to_owned(),
            json!([{
                "disposition":"omit", "authority_kind":"none", "category":"none",
                "type":"none", "lifecycle":"none", "proposition":"",
                "authority_source_node_id":"", "grounding_source_node_id":"",
                "reason":"No durable memory here."
            }]),
        );
    }
    Ok(json!({"turns": turns, "evidence_requests": []}))
}

pub(super) struct UserMemoryEndpoint;

impl GeneralEndpoint for UserMemoryEndpoint {
    fn model(&self) -> &str {
        "runtime-host-user-memory-test"
    }

    fn complete_json(
        &self,
        _system_prompt: &str,
        user_payload: &str,
        schema_name: &str,
        _schema: &Value,
    ) -> Result<Value, GeneralEndpointError> {
        match schema_name {
            "insomnia_authority_disposition_ledger" => {
                let payload: Value = serde_json::from_str(user_payload)
                    .map_err(|error| GeneralEndpointError::Failure(error.to_string()))?;
                let user_id = payload["turns"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .find(|turn| turn["role"] == "user")
                    .and_then(|turn| turn["id"].as_str())
                    .ok_or(GeneralEndpointError::InvalidResponse("missing user turn"))?;
                Ok(json!({
                    "turns": {user_id: [{
                        "disposition":"retain", "authority_kind":"direct", "category":"preference",
                        "type":"communication", "lifecycle":"current",
                        "proposition":"The user prefers Helix for editing code.",
                        "authority_source_node_id":"", "grounding_source_node_id":"",
                        "reason":"durable user-global preference"
                    }]},
                    "evidence_requests": []
                }))
            }
            "insomnia_memory_ownership" => Ok(json!({
                "groups": {"g000": {"ownership": "user"}}
            })),
            "insomnia_memory_wording" => Ok(json!({
                "groups": {"g000": {
                    "title":"Preferred editor",
                    "content":"The user prefers Helix for editing code."
                }}
            })),
            "insomnia_memory_entity_mentions" => routing_metadata_response(user_payload),
            _ => Err(GeneralEndpointError::Failure(format!(
                "unexpected schema {schema_name}"
            ))),
        }
    }
}

pub(super) struct OmitEndpoint;

impl GeneralEndpoint for OmitEndpoint {
    fn model(&self) -> &str {
        "runtime-host-test"
    }

    fn complete_json(
        &self,
        _system_prompt: &str,
        user_payload: &str,
        schema_name: &str,
        _schema: &Value,
    ) -> Result<Value, GeneralEndpointError> {
        if schema_name != "insomnia_authority_disposition_ledger" {
            return Err(GeneralEndpointError::Failure(format!(
                "unexpected schema {schema_name}"
            )));
        }
        omit_ledger(user_payload)
    }
}

pub(super) struct BlockingEndpoint {
    pub(super) entered: Arc<Barrier>,
    pub(super) release: Arc<Barrier>,
}

impl GeneralEndpoint for BlockingEndpoint {
    fn model(&self) -> &str {
        "runtime-host-blocking-test"
    }

    fn complete_json(
        &self,
        _system_prompt: &str,
        user_payload: &str,
        schema_name: &str,
        _schema: &Value,
    ) -> Result<Value, GeneralEndpointError> {
        self.entered.wait();
        self.release.wait();
        if schema_name != "insomnia_authority_disposition_ledger" {
            return Err(GeneralEndpointError::Failure(format!(
                "unexpected schema {schema_name}"
            )));
        }
        omit_ledger(user_payload)
    }
}

pub(super) struct BlockingEmbeddingEndpoint {
    pub(super) entered: Arc<Barrier>,
    pub(super) release: Arc<Barrier>,
    pub(super) block_once: AtomicBool,
}

impl BlockingEmbeddingEndpoint {
    pub(super) fn new(entered: Arc<Barrier>, release: Arc<Barrier>) -> Self {
        Self {
            entered,
            release,
            block_once: AtomicBool::new(true),
        }
    }
}

impl EmbeddingEndpoint for BlockingEmbeddingEndpoint {
    fn dimensions(&self) -> u32 {
        8
    }
    fn normalization(&self) -> VectorNormalization {
        VectorNormalization::L2
    }

    fn embed(
        &self,
        mode: EmbeddingMode,
        inputs: &[String],
    ) -> Result<Vec<Vec<f32>>, EmbeddingEndpointError> {
        if self.block_once.swap(false, Ordering::SeqCst) {
            self.entered.wait();
            self.release.wait();
        }
        SimulatedEmbeddingEndpoint::new(8, VectorNormalization::L2, 7).embed(mode, inputs)
    }
}

pub(super) fn one_worker() -> InsomniaWorkerConfig {
    InsomniaWorkerConfig {
        workers: 1,
        poll_interval_ns: 1_000_000,
        ..Default::default()
    }
}

pub(super) fn wait_complete(host: &ReliquaryRuntimeHost, target: usize) {
    wait_until(
        || host.insomnia_stats().unwrap().complete >= target,
        "Insomnia completion",
    );
}

pub(super) fn wait_memory(host: &ReliquaryRuntimeHost, target: usize) {
    wait_until(
        || host.memory_stats().unwrap().memories >= target,
        "Memory publication",
    );
}

pub(super) fn wait_vectors(host: &ReliquaryRuntimeHost, target: usize) {
    wait_until(
        || host.memory_vector_stats().unwrap().bindings >= target,
        "vectorization",
    );
}

pub(super) fn wait_memory_revisions(host: &ReliquaryRuntimeHost, target: usize) {
    wait_until(
        || host.memory_stats().unwrap().revisions >= target,
        "Reliquary Dream lifecycle",
    );
}

pub(super) fn wait_phylactery_memory(host: &ReliquaryRuntimeHost, target: usize) {
    wait_until(
        || {
            host.phylactery_memory_stats()
                .unwrap()
                .is_some_and(|stats| stats.memories >= target)
        },
        "Phylactery Memory publication",
    );
}

pub(super) fn wait_phylactery_vectors(host: &ReliquaryRuntimeHost, target: usize) {
    wait_until(
        || {
            host.phylactery_memory_vector_stats()
                .unwrap()
                .is_some_and(|stats| stats.bindings >= target)
        },
        "Phylactery vectorization",
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

pub(super) fn queue_memory_episode(cva: &mut Cva) {
    cva.append_node(
        "u0".into(),
        "c1".into(),
        None,
        "user".into(),
        1,
        "I prefer Helix for editing code.",
    )
    .unwrap();
    cva.finalize_import_path_and_queue("c1", "u0", EpisodeConfig::default(), 10)
        .unwrap();
}

pub(super) fn memory_endpoint() -> Arc<dyn GeneralEndpoint> {
    Arc::new(UserMemoryEndpoint)
}

fn routing_metadata_response(user_payload: &str) -> Result<Value, GeneralEndpointError> {
    let payload: Value = serde_json::from_str(user_payload)
        .map_err(|error| GeneralEndpointError::Failure(error.to_string()))?;
    let mut memories = serde_json::Map::new();
    for memory in payload["memories"].as_array().into_iter().flatten() {
        let key = memory["memory_key"]
            .as_str()
            .ok_or(GeneralEndpointError::InvalidResponse("missing Memory key"))?;
        let content = memory["content"].as_str().unwrap_or("");
        let mentions = if content.contains("Helix") {
            vec![json!({"field":"content", "text":"Helix", "occurrence":-1})]
        } else {
            Vec::new()
        };
        memories.insert(key.to_owned(), json!({"entity_mentions": mentions}));
    }
    Ok(json!({"memories": memories}))
}
