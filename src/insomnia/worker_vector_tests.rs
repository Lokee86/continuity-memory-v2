use crate::{
    Cva, EmbeddingEndpoint, EmbeddingEndpointError, EmbeddingMode, EpisodeConfig,
    InsomniaExtractor, InsomniaWorkerConfig, InsomniaWorkerError, Phylactery,
    SimulatedEmbeddingEndpoint, SimulatedGeneralEndpoint, VectorNormalization,
};
use serde_json::json;
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

fn test_path(name: &str) -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("continuity-insomnia-auto-vectors-{unique}"));
    fs::create_dir_all(&dir).unwrap();
    dir.join(name)
}

fn queue_memory_episode(cva: &mut Cva) {
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

fn memory_extractor() -> InsomniaExtractor<SimulatedGeneralEndpoint> {
    InsomniaExtractor::new(SimulatedGeneralEndpoint::new(
        "test-model",
        vec![
            json!({
                "turns": {"u0": [{
                    "disposition":"retain", "authority_kind":"direct", "category":"preference",
                    "type":"project", "lifecycle":"current",
                    "proposition":"The user prefers Helix for editing code.",
                    "authority_source_node_id":"", "grounding_source_node_id":"",
                    "reason":"durable preference"
                }]},
                "evidence_requests": []
            }),
            json!({"groups": {"g000": {
                "title":"Preferred editor",
                "content":"The user prefers Helix for editing code."
            }}}),
        ],
    ))
}

struct FailingEmbeddingEndpoint;

impl EmbeddingEndpoint for FailingEmbeddingEndpoint {
    fn dimensions(&self) -> u32 {
        8
    }

    fn normalization(&self) -> VectorNormalization {
        VectorNormalization::L2
    }

    fn embed(
        &self,
        _mode: EmbeddingMode,
        _inputs: &[String],
    ) -> Result<Vec<Vec<f32>>, EmbeddingEndpointError> {
        Err(EmbeddingEndpointError::Failure(
            "simulated embedding outage".into(),
        ))
    }
}

#[test]
fn backlog_drain_automatically_vectorizes_created_memories() {
    let path = test_path("automatic.cva");
    let mut cva = Cva::create(path).unwrap();
    queue_memory_episode(&mut cva);
    let extractor = memory_extractor();
    let embedding = SimulatedEmbeddingEndpoint::new(8, VectorNormalization::L2, 7);

    let result = cva
        .drain_insomnia_backlog(
            &extractor,
            &embedding,
            InsomniaWorkerConfig {
                workers: 1,
                ..Default::default()
            },
        )
        .unwrap();

    assert_eq!(result.completed_episodes, 1);
    assert_eq!(result.memories_created, 1);
    assert_eq!(result.memory_vectors_embedded, 1);
    assert_eq!(result.memory_vectors_already_present, 0);
    assert!(result.memory_vector_profile_id.is_some());
    assert!(result.memory_vector_set_id.is_some());
    assert_eq!(cva.memory_stats().memories, 1);
    assert_eq!(cva.memory_vector_stats().bindings, 1);
    let episode = cva.episodes().into_iter().next().unwrap();
    let memory_id = cva.insomnia_attempts(episode.id)[0].memory_ids[0];
    assert_eq!(cva.memory(memory_id).unwrap().authority_kind, "direct");
}

#[test]
fn routed_backlog_drain_vectorizes_user_memory_in_phylactery() {
    let path = test_path("routed.cva");
    let phy_path = test_path("routed.phy");
    let mut cva = Cva::create(path).unwrap();
    let mut phy = Phylactery::create(phy_path).unwrap();
    queue_memory_episode(&mut cva);
    let extractor = memory_extractor().with_ownership_endpoint(SimulatedGeneralEndpoint::new(
        "ownership-model",
        vec![json!({"groups": {"g000": {"ownership": "user"}}})],
    ));
    let embedding = SimulatedEmbeddingEndpoint::new(8, VectorNormalization::L2, 7);

    let result = cva
        .drain_insomnia_backlog_routed(
            &mut phy,
            &extractor,
            &embedding,
            InsomniaWorkerConfig {
                workers: 1,
                ..Default::default()
            },
        )
        .unwrap();

    assert_eq!(result.memories_created, 0);
    assert_eq!(result.user_memories_created, 1);
    assert_eq!(result.user_memory_vectors_embedded, 1);
    assert!(result.user_memory_vector_profile_id.is_some());
    assert_eq!(cva.memory_stats().memories, 0);
    assert_eq!(phy.memory_stats().memories, 1);
    assert_eq!(phy.memory_vector_stats().bindings, 1);
    let episode = cva.episodes().into_iter().next().unwrap();
    let attempt = &cva.insomnia_attempts(episode.id)[0];
    assert_eq!(
        attempt.external_memory_refs[0].owner_id,
        phy.owner_id().unwrap()
    );
}

#[test]
fn automatic_vectorization_reuses_the_profile_and_only_reembeds_for_a_new_profile() {
    let path = test_path("profiles.cva");
    let mut cva = Cva::create(path).unwrap();
    queue_memory_episode(&mut cva);
    let extractor = memory_extractor();
    let embedding_a = SimulatedEmbeddingEndpoint::new(8, VectorNormalization::L2, 7);
    let first = cva
        .drain_insomnia_backlog(
            &extractor,
            &embedding_a,
            InsomniaWorkerConfig {
                workers: 1,
                ..Default::default()
            },
        )
        .unwrap();
    assert_eq!(first.memory_vectors_embedded, 1);

    let idle_extractor =
        InsomniaExtractor::new(SimulatedGeneralEndpoint::new("test-model", Vec::new()));
    let same_profile = cva
        .drain_insomnia_backlog(
            &idle_extractor,
            &embedding_a,
            InsomniaWorkerConfig {
                workers: 1,
                ..Default::default()
            },
        )
        .unwrap();
    assert_eq!(same_profile.memory_vectors_embedded, 0);
    assert_eq!(same_profile.memory_vectors_already_present, 1);
    assert_eq!(cva.memory_vector_stats().bindings, 1);

    let embedding_b = SimulatedEmbeddingEndpoint::new(8, VectorNormalization::L2, 9001);
    let new_profile = cva
        .drain_insomnia_backlog(
            &idle_extractor,
            &embedding_b,
            InsomniaWorkerConfig {
                workers: 1,
                ..Default::default()
            },
        )
        .unwrap();
    assert_ne!(
        first.memory_vector_profile_id,
        new_profile.memory_vector_profile_id
    );
    assert_eq!(new_profile.memory_vectors_embedded, 1);
    assert_eq!(cva.memory_vector_stats().bindings, 2);
}

#[test]
fn embedding_failure_leaves_memory_authoritative_and_next_drain_repairs_vectors() {
    let path = test_path("repair.cva");
    let mut cva = Cva::create(path).unwrap();
    queue_memory_episode(&mut cva);
    let extractor = memory_extractor();

    let error = cva
        .drain_insomnia_backlog(
            &extractor,
            &FailingEmbeddingEndpoint,
            InsomniaWorkerConfig {
                workers: 1,
                ..Default::default()
            },
        )
        .unwrap_err();
    assert!(matches!(
        error,
        InsomniaWorkerError::CompatibilityProfile(_)
    ));
    assert_eq!(cva.memory_stats().memories, 1);
    assert_eq!(cva.insomnia_stats().complete, 1);
    assert_eq!(cva.memory_vector_stats().bindings, 0);

    let idle_extractor =
        InsomniaExtractor::new(SimulatedGeneralEndpoint::new("test-model", Vec::new()));
    let embedding = SimulatedEmbeddingEndpoint::new(8, VectorNormalization::L2, 7);
    let repaired = cva
        .drain_insomnia_backlog(
            &idle_extractor,
            &embedding,
            InsomniaWorkerConfig {
                workers: 1,
                ..Default::default()
            },
        )
        .unwrap();

    assert_eq!(repaired.claimed_attempts, 0);
    assert_eq!(repaired.memory_vectors_embedded, 1);
    assert_eq!(cva.memory_stats().memories, 1);
    assert_eq!(cva.memory_vector_stats().bindings, 1);
}
