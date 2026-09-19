use crate::dream_candidate_test_support::{install_vectors, memory_extracted, test_path};
use crate::{
    Cva, DreamProcessor, InsomniaExtractor, InsomniaWorkerConfig, InteractionRuntime,
    RuntimeBackgroundConfig, SimulatedEmbeddingEndpoint, SimulatedGeneralEndpoint,
    VectorNormalization,
};
use serde_json::json;

fn one_worker_config() -> RuntimeBackgroundConfig {
    RuntimeBackgroundConfig {
        insomnia: InsomniaWorkerConfig {
            workers: 1,
            ..Default::default()
        },
        ..Default::default()
    }
}

fn queue_preference_episode(cva: &mut Cva) {
    cva.append_node(
        "u0".into(),
        "c1".into(),
        None,
        "user".into(),
        1,
        "I prefer Helix for editing code.",
    )
    .unwrap();
    cva.finalize_import_path_and_queue("c1", "u0", crate::EpisodeConfig::default(), 10)
        .unwrap();
}

fn preference_extractor() -> InsomniaExtractor<SimulatedGeneralEndpoint> {
    InsomniaExtractor::new(SimulatedGeneralEndpoint::new(
        "insomnia",
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

#[test]
fn one_runtime_cycle_drains_insomnia_then_processes_new_memory_with_dream() {
    let mut cva = Cva::create(test_path("background-insomnia-dream.cva")).unwrap();
    queue_preference_episode(&mut cva);
    let mut runtime = InteractionRuntime::new(cva);
    let extractor = preference_extractor();
    let embedding = SimulatedEmbeddingEndpoint::new(8, VectorNormalization::L2, 7);
    let dream = DreamProcessor::new(
        SimulatedGeneralEndpoint::new("dream-classifier", vec![]),
        SimulatedGeneralEndpoint::new("dream-verifier", vec![]),
    );

    let result = runtime
        .process_background_work(&extractor, &embedding, &dream, one_worker_config())
        .unwrap();

    assert_eq!(result.insomnia.completed_episodes, 1);
    assert_eq!(result.insomnia.memories_created, 1);
    assert_eq!(result.insomnia.memory_vectors_embedded, 1);
    assert_eq!(result.dream_attempted, 1);
    assert_eq!(result.dream_completed.len(), 1);
    assert!(result.dream_failures.is_empty());
    assert_eq!(result.dream_remaining, 0);
    assert_eq!(result.dream_completed[0].candidate_count, 0);
    assert_eq!(result.dream_completed[0].lifecycle_state, "canonical");

    let memory_id = result.dream_completed[0].memory_id;
    let mut cva = runtime.into_cva();
    let memory = cva.memory(memory_id).unwrap();
    assert_eq!(memory.lifecycle_state, "canonical");
    assert_eq!(memory.authority_kind, "direct");
}

#[test]
fn completed_dream_work_is_not_requeued_by_the_shared_runtime() {
    let mut cva = Cva::create(test_path("background-idempotent.cva")).unwrap();
    queue_preference_episode(&mut cva);
    let mut runtime = InteractionRuntime::new(cva);
    let extractor = preference_extractor();
    let embedding = SimulatedEmbeddingEndpoint::new(8, VectorNormalization::L2, 7);
    let dream = DreamProcessor::new(
        SimulatedGeneralEndpoint::new("dream-classifier", vec![]),
        SimulatedGeneralEndpoint::new("dream-verifier", vec![]),
    );

    let first = runtime
        .process_background_work(&extractor, &embedding, &dream, one_worker_config())
        .unwrap();
    assert_eq!(first.dream_attempted, 1);

    let second = runtime
        .process_background_work(&extractor, &embedding, &dream, one_worker_config())
        .unwrap();
    assert_eq!(second.insomnia.claimed_attempts, 0);
    assert_eq!(second.dream_attempted, 0);
    assert_eq!(second.dream_remaining, 0);
}

#[test]
fn one_dream_failure_does_not_block_later_pending_memory_in_the_same_cycle() {
    let mut cva = Cva::create(test_path("background-failure-isolation.cva")).unwrap();
    let first = memory_extracted(&mut cva, "first", "Same subject", "Alpha state.", 100);
    let second = memory_extracted(&mut cva, "second", "Same subject", "Beta state.", 110);
    install_vectors(&mut cva, &[first, second], &[&[1.0, 0.0], &[0.95, 0.05]]);
    let mut runtime = InteractionRuntime::new(cva);
    let extractor = InsomniaExtractor::new(SimulatedGeneralEndpoint::new("insomnia", vec![]));
    let embedding = SimulatedEmbeddingEndpoint::new(2, VectorNormalization::L2, 41);
    let dream = DreamProcessor::new(
        SimulatedGeneralEndpoint::new(
            "dream-classifier",
            vec![
                json!({}),
                json!({"relation":"none", "direction":"none", "evidence":[]}),
            ],
        ),
        SimulatedGeneralEndpoint::new("dream-verifier", vec![]),
    );

    let result = runtime
        .process_background_work(&extractor, &embedding, &dream, one_worker_config())
        .unwrap();

    assert_eq!(result.dream_attempted, 2);
    assert_eq!(result.dream_failures.len(), 1);
    assert_eq!(result.dream_completed.len(), 1);
    assert_eq!(result.dream_remaining, 1);

    let failed_id = result.dream_failures[0].memory_id;
    let completed_id = result.dream_completed[0].memory_id;
    let mut cva = runtime.into_cva();
    assert_eq!(cva.memory(failed_id).unwrap().lifecycle_state, "extracted");
    assert_eq!(
        cva.memory(completed_id).unwrap().lifecycle_state,
        "knowledge"
    );
}
