use super::test_support::{episode, test_path};
use crate::{
    Cva, EpisodeBoundary, EpisodeConfig, EpisodeOrigin, EpisodePolicy, InsomniaError,
    InsomniaPriority, InsomniaWorkState,
};

#[test]
fn scheduling_priority_is_immediate_then_live_then_import() {
    let path = test_path("priority.cva");
    let mut cva = Cva::create(&path).unwrap();
    let imported = episode(
        &mut cva,
        "imported",
        "i",
        10,
        EpisodeOrigin::Import,
        EpisodeBoundary::ImportEnd,
    );
    let live = episode(
        &mut cva,
        "live",
        "l",
        100,
        EpisodeOrigin::Live,
        EpisodeBoundary::Inactivity,
    );
    let immediate = episode(
        &mut cva,
        "immediate",
        "m",
        200,
        EpisodeOrigin::Live,
        EpisodeBoundary::CreateMemory,
    );
    cva.queue_insomnia_episode(imported.id, InsomniaPriority::Import, 300)
        .unwrap();
    cva.queue_insomnia_episode(live.id, InsomniaPriority::Live, 300)
        .unwrap();
    cva.queue_insomnia_episode(immediate.id, InsomniaPriority::ImmediateLive, 300)
        .unwrap();

    let first = cva.claim_insomnia_episode("w1", 301, 100).unwrap().unwrap();
    assert_eq!(first.episode_id, immediate.id);
    cva.complete_insomnia_episode(
        first.episode_id,
        first.lease_token.unwrap(),
        301,
        302,
        "model".into(),
        "v1".into(),
        vec![],
        0,
    )
    .unwrap();

    let second = cva.claim_insomnia_episode("w1", 303, 100).unwrap().unwrap();
    assert_eq!(second.episode_id, live.id);
    cva.complete_insomnia_episode(
        second.episode_id,
        second.lease_token.unwrap(),
        303,
        304,
        "model".into(),
        "v1".into(),
        vec![],
        0,
    )
    .unwrap();

    let third = cva.claim_insomnia_episode("w1", 305, 100).unwrap().unwrap();
    assert_eq!(third.episode_id, imported.id);
}

#[test]
fn create_memory_closes_only_the_tail_and_queues_it_immediately() {
    let path = test_path("create-memory.cva");
    let mut cva = Cva::create(&path).unwrap();
    cva.append_node(
        "u0".into(),
        "c1".into(),
        None,
        "user".into(),
        10,
        "Please remember this.",
    )
    .unwrap();
    cva.append_node(
        "a0".into(),
        "c1".into(),
        Some("u0".into()),
        "assistant".into(),
        20,
        "ack",
    )
    .unwrap();

    let result = cva
        .request_create_memory("c1", "a0", EpisodeConfig::default(), 30)
        .unwrap();
    assert_eq!(result.episodes.created.len(), 1);
    assert_eq!(result.queued.len(), 1);
    assert_eq!(result.queued[0].priority, InsomniaPriority::ImmediateLive);
    assert_eq!(result.queued[0].state, InsomniaWorkState::Pending);

    cva.append_node(
        "u1".into(),
        "c1".into(),
        Some("a0".into()),
        "user".into(),
        40,
        "continue",
    )
    .unwrap();
    cva.append_node(
        "a1".into(),
        "c1".into(),
        Some("u1".into()),
        "assistant".into(),
        50,
        "continued",
    )
    .unwrap();
    let normal = cva
        .finalize_inactive_path_and_queue(
            "c1",
            "a1",
            EpisodePolicy {
                episode: EpisodeConfig::default(),
                inactivity_ns: 10,
            },
            60,
        )
        .unwrap()
        .unwrap();
    assert_eq!(normal.episodes.created[0].start_node_id, "u1");
    assert_eq!(normal.queued[0].priority, InsomniaPriority::Live);
}

#[test]
fn stale_lease_cannot_finalize_reclaimed_work() {
    let path = test_path("lease.cva");
    let mut cva = Cva::create(&path).unwrap();
    let episode = episode(
        &mut cva,
        "c1",
        "e",
        10,
        EpisodeOrigin::Live,
        EpisodeBoundary::Inactivity,
    );
    cva.queue_insomnia_episode(episode.id, InsomniaPriority::Live, 20)
        .unwrap();
    let first = cva.claim_insomnia_episode("w1", 30, 10).unwrap().unwrap();
    let old_token = first.lease_token.unwrap();
    let second = cva.claim_insomnia_episode("w2", 41, 10).unwrap().unwrap();
    assert_eq!(second.attempt_count, 2);
    assert_ne!(second.lease_token, Some(old_token));
    assert!(matches!(
        cva.complete_insomnia_episode(
            episode.id,
            old_token,
            30,
            42,
            "model".into(),
            "v1".into(),
            vec![],
            0,
        ),
        Err(InsomniaError::InvalidLease)
    ));
}
