use super::test_support::{episode, test_path};
use crate::{Cva, EpisodeBoundary, EpisodeOrigin, InsomniaPriority, InsomniaWorkState};

#[test]
fn processing_claims_are_reclaimable_after_restart_and_attempt_history_persists() {
    let path = test_path("restart.cva");
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
    let first = cva.claim_insomnia_episode("w1", 30, 1000).unwrap().unwrap();
    assert_eq!(first.state, InsomniaWorkState::Processing);
    cva.sync().unwrap();
    drop(cva);

    let mut reopened = Cva::open(&path).unwrap();
    assert_eq!(
        reopened.insomnia_work(episode.id).unwrap().state,
        InsomniaWorkState::Pending
    );
    let reclaimed = reopened
        .claim_insomnia_episode("w2", 31, 1000)
        .unwrap()
        .unwrap();
    reopened
        .complete_insomnia_episode(
            episode.id,
            reclaimed.lease_token.unwrap(),
            31,
            32,
            "model".into(),
            "v1".into(),
            vec![],
            0,
        )
        .unwrap();
    reopened.sync().unwrap();
    drop(reopened);

    let final_open = Cva::open(&path).unwrap();
    assert_eq!(final_open.insomnia_attempts(episode.id).len(), 1);
    assert_eq!(
        final_open.insomnia_work(episode.id).unwrap().state,
        InsomniaWorkState::Complete
    );
}

#[test]
fn retry_delay_is_durable_and_respected() {
    let path = test_path("retry.cva");
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
    let claim = cva.claim_insomnia_episode("w1", 30, 100).unwrap().unwrap();
    cva.fail_insomnia_episode(
        episode.id,
        claim.lease_token.unwrap(),
        30,
        31,
        50,
        "provider overloaded".into(),
    )
    .unwrap();
    assert!(cva.claim_insomnia_episode("w2", 49, 100).unwrap().is_none());
    cva.sync().unwrap();
    drop(cva);

    let mut reopened = Cva::open(&path).unwrap();
    assert!(
        reopened
            .claim_insomnia_episode("w2", 49, 100)
            .unwrap()
            .is_none()
    );
    assert!(
        reopened
            .claim_insomnia_episode("w2", 50, 100)
            .unwrap()
            .is_some()
    );
}
