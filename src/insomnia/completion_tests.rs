use super::completion::decode_completion;
use crate::{
    Cva, EpisodeBoundary, EpisodeConfig, EpisodeOrigin, InsomniaExtractor, InsomniaPriority,
    InsomniaWorkState, SimulatedGeneralEndpoint,
};
use serde_json::{Value, json};
use std::fs::OpenOptions;

use super::test_support::test_path;

fn setup(name: &str) -> (std::path::PathBuf, Cva, crate::Episode) {
    let path = test_path(name);
    let mut cva = Cva::create(&path).unwrap();
    let mut parent = None;
    for index in 0..8 {
        let user = format!("u{index}");
        let assistant = format!("a{index}");
        cva.append_node(
            user.clone(),
            "c1".into(),
            parent.clone(),
            "user".into(),
            10 + index as i64 * 2,
            &format!("Atomic Insomnia completion fact {index}."),
        )
        .unwrap();
        cva.append_node(
            assistant.clone(),
            "c1".into(),
            Some(user),
            "assistant".into(),
            11 + index as i64 * 2,
            "Acknowledged.",
        )
        .unwrap();
        parent = Some(assistant);
    }
    let episode = cva
        .materialize_path_episodes(
            "c1",
            parent.as_deref().unwrap(),
            EpisodeConfig::default(),
            EpisodeOrigin::Live,
            Some((EpisodeBoundary::Inactivity, 100)),
        )
        .unwrap()
        .created
        .remove(0);
    cva.queue_insomnia_episode(episode.id, InsomniaPriority::Live, 101)
        .unwrap();
    (path, cva, episode)
}

fn response(count: usize) -> Value {
    let candidates: Vec<_> = (0..count)
        .map(|index| {
            json!({
                "authority_kind": "direct",
                "category": "decision",
                "type": "project",
                "title": format!("Atomic completion {index}"),
                "content": format!("Atomic Insomnia completion invariant {index}."),
                "source_node_id": format!("u{index}"),
                "source_quote": format!("Atomic Insomnia completion fact {index}."),
                "authority_source_conversation_id": "",
                "authority_source_node_id": "",
                "authority_source_quote": "",
                "grounding_source_conversation_id": "",
                "grounding_source_node_id": "",
                "grounding_source_quote": ""
            })
        })
        .collect();
    json!({"candidates": candidates})
}

fn process(cva: &mut Cva, count: usize, now: i64) -> crate::InsomniaProcessResult {
    let claim = cva
        .claim_insomnia_episode("worker", now, 1_000)
        .unwrap()
        .unwrap();
    let extractor = InsomniaExtractor::new(SimulatedGeneralEndpoint::new(
        "synthetic-insomnia",
        vec![response(count)],
    ));
    cva.process_claimed_insomnia_episode(&claim, &extractor, "private", now, now + 1)
        .unwrap()
}

#[test]
fn successful_completion_collapses_retry_history_and_reopens() {
    let (path, mut cva, episode) = setup("completion-retry.cva");
    let first = cva
        .claim_insomnia_episode("worker", 110, 1_000)
        .unwrap()
        .unwrap();
    cva.fail_insomnia_episode(
        episode.id,
        first.lease_token.unwrap(),
        110,
        111,
        120,
        "synthetic retry".into(),
    )
    .unwrap();
    assert!(cva.insomnia_attempts(episode.id).is_empty());
    let result = process(&mut cva, 2, 120);
    assert_eq!(result.created.len(), 2);
    assert_eq!(cva.insomnia_attempts(episode.id).len(), 1);
    assert_eq!(cva.insomnia_attempts(episode.id)[0].attempt, 2);
    cva.sync().unwrap();
    drop(cva);

    let reopened = Cva::open(&path).unwrap();
    assert_eq!(reopened.memory_stats().memories, 2);
    let attempts = reopened.insomnia_attempts(episode.id);
    assert_eq!(attempts.len(), 1);
    assert_eq!(attempts[0].attempt, 2);
    assert_eq!(attempts[0].state, InsomniaWorkState::Complete);
    assert_eq!(attempts[0].extractor_model, "synthetic-insomnia");
    assert_eq!(
        reopened.insomnia_work(episode.id).unwrap().state,
        InsomniaWorkState::Complete
    );
}

#[test]
fn zero_and_many_memory_completions_are_single_outcomes() {
    for (name, count) in [("completion-zero.cva", 0), ("completion-many.cva", 8)] {
        let (path, mut cva, episode) = setup(name);
        let result = process(&mut cva, count, 110);
        assert_eq!(result.created.len(), count);
        drop(cva);
        let reopened = Cva::open(&path).unwrap();
        assert_eq!(reopened.memory_stats().memories, count);
        let attempts = reopened.insomnia_attempts(episode.id);
        assert_eq!(attempts.len(), 1);
        assert_eq!(attempts[0].memory_ids.len(), count);
    }
}

#[test]
fn truncated_completion_is_recovered_as_fully_unpublished_then_retries_cleanly() {
    let (path, mut cva, episode) = setup("completion-truncated.cva");
    let result = process(&mut cva, 2, 110);
    assert_eq!(result.created.len(), 2);
    let commit = cva
        .container
        .chunks()
        .unwrap()
        .into_iter()
        .find(|chunk| {
            let payload = cva.container.read(*chunk).unwrap();
            decode_completion(&payload).unwrap().is_some()
        })
        .unwrap();
    drop(cva);

    let partial_len = commit.offset + 8 + commit.len / 2;
    OpenOptions::new()
        .write(true)
        .open(&path)
        .unwrap()
        .set_len(partial_len)
        .unwrap();

    let mut reopened = Cva::open(&path).unwrap();
    assert_eq!(reopened.memory_stats().memories, 0);
    assert_eq!(
        reopened.insomnia_work(episode.id).unwrap().state,
        InsomniaWorkState::Pending
    );
    assert!(reopened.insomnia_attempts(episode.id).is_empty());
    let retry = process(&mut reopened, 2, 200);
    assert_eq!(retry.created.len(), 2);
    drop(reopened);

    let final_open = Cva::open(&path).unwrap();
    assert_eq!(final_open.memory_stats().memories, 2);
    assert_eq!(final_open.insomnia_attempts(episode.id).len(), 1);
    assert_eq!(
        final_open.insomnia_work(episode.id).unwrap().state,
        InsomniaWorkState::Complete
    );
}
