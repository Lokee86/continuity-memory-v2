use crate::{
    Cva, EpisodeBoundary, EpisodeConfig, EpisodeOrigin, INSOMNIA_SYSTEM_PROMPT, InsomniaExtractor,
    InsomniaPriority, InsomniaWorkState, SimulatedGeneralEndpoint,
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
    let dir = std::env::temp_dir().join(format!("continuity-insomnia-extract-{unique}"));
    fs::create_dir_all(&dir).unwrap();
    dir.join(name)
}

fn append(cva: &mut Cva, id: &str, parent: Option<&str>, role: &str, time: i64, content: &str) {
    cva.append_node(
        id.into(),
        "c1".into(),
        parent.map(str::to_owned),
        role.into(),
        time,
        content,
    )
    .unwrap();
}

fn queue_episode(cva: &mut Cva, leaf: &str) -> crate::Episode {
    let episode = cva
        .materialize_path_episodes(
            "c1",
            leaf,
            EpisodeConfig::default(),
            EpisodeOrigin::Live,
            Some((EpisodeBoundary::Inactivity, 100)),
        )
        .unwrap()
        .created
        .into_iter()
        .next()
        .unwrap();
    cva.queue_insomnia_episode(episode.id, InsomniaPriority::Live, 101)
        .unwrap();
    episode
}

#[test]
fn claimed_episode_extracts_and_publishes_authoritative_memory() {
    let path = test_path("publish.cva");
    let mut cva = Cva::create(&path).unwrap();
    append(
        &mut cva,
        "u0",
        None,
        "user",
        10,
        "Continuity should keep all working-memory generation in Insomnia.",
    );
    append(&mut cva, "a0", Some("u0"), "assistant", 20, "Understood.");
    let episode = queue_episode(&mut cva, "a0");
    let claim = cva
        .claim_insomnia_episode("worker", 110, 100)
        .unwrap()
        .unwrap();
    let endpoint = SimulatedGeneralEndpoint::new(
        "test-model",
        vec![json!({
            "candidates": [{
                "category": "decision",
                "type": "project",
                "title": "Insomnia owns working-memory generation",
                "content": "Continuity uses Insomnia as the sole authoritative generator of working memory.",
                "source_node_id": "u0",
                "source_quote": "Continuity should keep all working-memory generation in Insomnia.",
                "content_source_conversation_id": "",
                "content_source_node_id": "",
                "content_source_quote": ""
            }]
        })],
    );
    let extractor = InsomniaExtractor::new(endpoint);
    let result = cva
        .process_claimed_insomnia_episode(&claim, &extractor, "private", 110, 120)
        .unwrap();
    assert_eq!(result.created.len(), 1);
    assert!(result.existing.is_empty());
    assert!(result.rejected.is_empty());
    let memory = &result.created[0];
    assert_eq!(memory.source_episode_id, Some(episode.id));
    assert_eq!(memory.source_node_id.as_deref(), Some("u0"));
    assert_eq!(
        cva.insomnia_work(episode.id).unwrap().state,
        InsomniaWorkState::Complete
    );

    cva.sync().unwrap();
    drop(cva);
    let reopened = Cva::open(&path).unwrap();
    assert_eq!(reopened.memory_stats().memories, 1);
    assert_eq!(reopened.insomnia_attempts(episode.id).len(), 1);
}

#[test]
fn assistant_cannot_become_user_authority() {
    let path = test_path("authority.cva");
    let mut cva = Cva::create(&path).unwrap();
    append(&mut cva, "u0", None, "user", 10, "What should we do?");
    append(
        &mut cva,
        "a0",
        Some("u0"),
        "assistant",
        20,
        "Use Insomnia for all memory generation.",
    );
    let episode = queue_episode(&mut cva, "a0");
    let claim = cva.claim_insomnia_episode("w", 110, 100).unwrap().unwrap();
    let extractor = InsomniaExtractor::new(SimulatedGeneralEndpoint::new(
        "test-model",
        vec![json!({"candidates": [{
            "category": "decision",
            "type": "project",
            "title": "Bad authority",
            "content": "Use Insomnia.",
            "source_node_id": "a0",
            "source_quote": "Use Insomnia for all memory generation.",
            "content_source_conversation_id": "",
            "content_source_node_id": "",
            "content_source_quote": ""
        }]})],
    ));
    let result = cva
        .process_claimed_insomnia_episode(&claim, &extractor, "private", 110, 120)
        .unwrap();
    assert!(result.created.is_empty());
    assert_eq!(result.rejected.len(), 1);
    assert_eq!(cva.memory_stats().memories, 0);
    assert_eq!(
        cva.insomnia_work(episode.id).unwrap().state,
        InsomniaWorkState::Complete
    );
}

#[test]
fn explicit_adoption_can_use_assistant_content_with_user_authority() {
    let path = test_path("adoption.cva");
    let mut cva = Cva::create(&path).unwrap();
    append(
        &mut cva,
        "u0",
        None,
        "user",
        10,
        "What should the episode policy be?",
    );
    append(
        &mut cva,
        "a0",
        Some("u0"),
        "assistant",
        20,
        "Use a fifteen minute inactivity boundary.",
    );
    append(&mut cva, "u1", Some("a0"), "user", 30, "Remember that.");
    append(&mut cva, "a1", Some("u1"), "assistant", 40, "Recorded.");
    let episode = queue_episode(&mut cva, "a1");
    let claim = cva.claim_insomnia_episode("w", 110, 100).unwrap().unwrap();
    let extractor = InsomniaExtractor::new(SimulatedGeneralEndpoint::new(
        "test-model",
        vec![json!({"candidates": [{
            "category": "decision",
            "type": "project",
            "title": "Episode inactivity boundary",
            "content": "Episodes use a fifteen minute inactivity boundary.",
            "source_node_id": "u1",
            "source_quote": "Remember that.",
            "content_source_conversation_id": "c1",
            "content_source_node_id": "a0",
            "content_source_quote": "Use a fifteen minute inactivity boundary."
        }]})],
    ));
    let result = cva
        .process_claimed_insomnia_episode(&claim, &extractor, "private", 110, 120)
        .unwrap();
    assert_eq!(result.created.len(), 1);
    assert_eq!(
        result.created[0].content_source_node_id.as_deref(),
        Some("a0")
    );
    assert_eq!(
        result.created[0].content_source_conversation_id.as_deref(),
        Some("c1")
    );
    assert_eq!(result.created[0].source_episode_id, Some(episode.id));
}

#[test]
fn explicit_retention_rule_remains_in_extractor_contract() {
    assert!(
        INSOMNIA_SYSTEM_PROMPT.contains("Explicit imperative retention is a strong requirement")
    );
    assert!(INSOMNIA_SYSTEM_PROMPT.contains("remember this"));
    assert!(INSOMNIA_SYSTEM_PROMPT.contains("Do you remember that?"));
    assert!(INSOMNIA_SYSTEM_PROMPT.contains("MUST NOT be treated as retention instructions"));
}

#[test]
fn urgent_scheduling_does_not_force_a_memory() {
    let path = test_path("zero.cva");
    let mut cva = Cva::create(&path).unwrap();
    append(&mut cva, "u0", None, "user", 10, "hello");
    append(&mut cva, "a0", Some("u0"), "assistant", 20, "hi");
    let scheduled = cva
        .request_create_memory("c1", "a0", EpisodeConfig::default(), 30)
        .unwrap();
    let episode = scheduled.episodes.created[0].clone();
    let claim = cva.claim_insomnia_episode("w", 31, 100).unwrap().unwrap();
    let extractor = InsomniaExtractor::new(SimulatedGeneralEndpoint::new(
        "test-model",
        vec![json!({"candidates": []})],
    ));
    let result = cva
        .process_claimed_insomnia_episode(&claim, &extractor, "private", 31, 32)
        .unwrap();
    assert!(result.created.is_empty());
    assert_eq!(cva.memory_stats().memories, 0);
    assert_eq!(
        cva.insomnia_work(episode.id).unwrap().state,
        InsomniaWorkState::Complete
    );
}
