use super::test_support::{episode, test_path};
use crate::{Cva, EpisodeBoundary, EpisodeOrigin, InsomniaPriority, InsomniaWorkState};

#[test]
fn completed_history_is_not_kept_in_ready_scheduler() {
    let path = test_path("completed-history.cva");
    let mut cva = Cva::create(&path).unwrap();
    for index in 0..128 {
        let item = episode(
            &mut cva,
            &format!("done-{index}"),
            &format!("d{index}"),
            10 + index as i64 * 10,
            EpisodeOrigin::Import,
            EpisodeBoundary::ImportEnd,
        );
        cva.queue_insomnia_episode(item.id, InsomniaPriority::Import, 10_000)
            .unwrap();
        let claim = cva
            .claim_insomnia_episode("history", 20_000 + index as i64, 1_000)
            .unwrap()
            .unwrap();
        cva.complete_insomnia_episode(
            item.id,
            claim.lease_token.unwrap(),
            20_000 + index as i64,
            20_001 + index as i64,
            "model".into(),
            "v1".into(),
            vec![],
            0,
        )
        .unwrap();
    }
    let pending = episode(
        &mut cva,
        "pending",
        "p",
        50_000,
        EpisodeOrigin::Live,
        EpisodeBoundary::Inactivity,
    );
    cva.queue_insomnia_episode(pending.id, InsomniaPriority::Live, 50_010)
        .unwrap();

    assert_eq!(cva.insomnia_stats().total, 129);
    assert_eq!(cva.insomnia_stats().complete, 128);
    assert_eq!(cva.insomnia.scheduler_counts(), (1, 0, 0));
}

#[test]
fn same_priority_claims_preserve_source_chronology() {
    let path = test_path("chronology.cva");
    let mut cva = Cva::create(&path).unwrap();
    let late = episode(
        &mut cva,
        "late",
        "late",
        300,
        EpisodeOrigin::Live,
        EpisodeBoundary::Inactivity,
    );
    let early = episode(
        &mut cva,
        "early",
        "early",
        100,
        EpisodeOrigin::Live,
        EpisodeBoundary::Inactivity,
    );
    let middle = episode(
        &mut cva,
        "middle",
        "middle",
        200,
        EpisodeOrigin::Live,
        EpisodeBoundary::Inactivity,
    );
    for item in [&late, &middle, &early] {
        cva.queue_insomnia_episode(item.id, InsomniaPriority::Live, 1_000)
            .unwrap();
    }

    let first = cva
        .claim_insomnia_episode("w", 1_001, 100)
        .unwrap()
        .unwrap();
    assert_eq!(first.episode_id, early.id);
    assert_eq!(first.state, InsomniaWorkState::Processing);
}
