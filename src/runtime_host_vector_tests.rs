use crate::runtime_host_test_support::{
    BlockingEmbeddingEndpoint, memory_endpoint, one_worker, queue_memory_episode, test_path,
    wait_memory, wait_memory_revisions, wait_vectors,
};
use crate::{
    Cva, EpisodePolicy, InteractionRole, InteractionRuntime, ReliquaryRuntimeHost,
    SimulatedEmbeddingEndpoint, VectorNormalization,
};
use std::sync::{Arc, Barrier};

#[test]
fn embedding_probe_does_not_hold_the_interaction_runtime_lock() {
    let cva = Cva::create(test_path("embedding-concurrent.cva")).unwrap();
    let entered = Arc::new(Barrier::new(2));
    let release = Arc::new(Barrier::new(2));
    let endpoint = Arc::new(BlockingEmbeddingEndpoint::new(
        Arc::clone(&entered),
        Arc::clone(&release),
    ));
    let host = ReliquaryRuntimeHost::start(
        InteractionRuntime::new(cva),
        None,
        Some(endpoint),
        one_worker(),
        EpisodePolicy::default(),
    );

    entered.wait();
    host.open_session("live".into(), None).unwrap();
    host.begin_message("live", "u1".into(), InteractionRole::User, 1)
        .unwrap();
    host.append_text("live", "u1", "Embedding does not block chat.")
        .unwrap();
    host.complete_message("live", "u1").unwrap();
    release.wait();
    host.into_cva().unwrap();
}

#[test]
fn embedding_route_can_be_attached_after_memory_creation() {
    let mut cva = Cva::create(test_path("late-embedding.cva")).unwrap();
    queue_memory_episode(&mut cva);
    let host = ReliquaryRuntimeHost::start(
        InteractionRuntime::new(cva),
        Some(memory_endpoint()),
        None,
        one_worker(),
        EpisodePolicy::default(),
    );

    wait_memory(&host, 1);
    assert_eq!(host.memory_vector_stats().unwrap().bindings, 0);
    host.set_embedding_endpoint(Some(Arc::new(SimulatedEmbeddingEndpoint::new(
        8,
        VectorNormalization::L2,
        7,
    ))))
    .unwrap();
    wait_vectors(&host, 1);
    wait_memory_revisions(&host, 2);
    assert_eq!(host.memory_vector_stats().unwrap().bindings, 1);
    let mut cva = host.into_cva().unwrap();
    let id = cva.memory_ids()[0];
    assert_eq!(cva.memory(id).unwrap().lifecycle_state, "canonical");
}
