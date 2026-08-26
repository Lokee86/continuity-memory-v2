use crate::runtime_host_test_support::{
    BlockingEndpoint, OmitEndpoint, one_worker, test_path, wait_complete,
};
use crate::{
    Cva, EpisodeConfig, EpisodePolicy, InteractionRole, InteractionRuntime, ReliquaryRuntimeHost,
};
use std::sync::{Arc, Barrier};

#[test]
fn inference_does_not_hold_the_interaction_runtime_lock() {
    let mut cva = Cva::create(test_path("concurrent.cva")).unwrap();
    cva.append_node(
        "u0".into(),
        "import".into(),
        None,
        "user".into(),
        1,
        "No durable memory here.",
    )
    .unwrap();
    cva.finalize_import_path_and_queue("import", "u0", EpisodeConfig::default(), 2)
        .unwrap();

    let entered = Arc::new(Barrier::new(2));
    let release = Arc::new(Barrier::new(2));
    let endpoint = Arc::new(BlockingEndpoint {
        entered: Arc::clone(&entered),
        release: Arc::clone(&release),
    });
    let host = ReliquaryRuntimeHost::start(
        InteractionRuntime::new(cva),
        Some(endpoint),
        None,
        one_worker(),
        EpisodePolicy {
            episode: EpisodeConfig::default(),
            inactivity_ns: i64::MAX,
        },
    );

    entered.wait();
    host.open_session("live".into(), None).unwrap();
    host.begin_message("live", "u1".into(), InteractionRole::User, 3)
        .unwrap();
    host.append_text("live", "u1", "The interface remains usable.")
        .unwrap();
    host.complete_message("live", "u1").unwrap();
    release.wait();

    wait_complete(&host, 1);
    assert_eq!(host.into_cva().unwrap().insomnia_stats().complete, 1);
}

#[test]
fn live_completion_wakes_idle_insomnia_workers() {
    let cva = Cva::create(test_path("wake.cva")).unwrap();
    let policy = EpisodePolicy {
        episode: EpisodeConfig { max_input_bytes: 1 },
        inactivity_ns: 60_000_000_000,
    };
    let host = ReliquaryRuntimeHost::start(
        InteractionRuntime::new(cva),
        Some(Arc::new(OmitEndpoint)),
        None,
        one_worker(),
        policy,
    );

    host.open_session("live".into(), None).unwrap();
    host.begin_message("live", "u1".into(), InteractionRole::User, 1)
        .unwrap();
    host.append_text("live", "u1", "Question").unwrap();
    host.complete_message("live", "u1").unwrap();
    host.begin_message("live", "a1".into(), InteractionRole::Agent, 2)
        .unwrap();
    host.append_text("live", "a1", "Answer").unwrap();
    host.complete_live_message("live", "a1", policy, 3).unwrap();

    wait_complete(&host, 1);
    assert!(host.insomnia_stats().unwrap().complete >= 1);
    host.into_cva().unwrap();
}
