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
                "authority_kind": "direct",
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
            "authority_kind": "direct",
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
            "authority_kind": "retention",
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
fn legacy_extraction_policy_remains_in_extractor_contract() {
    for clause in [
        "Do not retain advice, recommendations, examples, explanations, generated copy",
        "Optimize for durable information density, not sentence-level atomicity",
        "Project implementation details, reusable commands and paths, procedures, sequencing, current state, diagnoses, and unresolved next actions may be durable",
        "Never emit a standalone memory whose durable content is only that a prompt, phase, step, migration batch, test run, commit, push, merge, file move, verification, or cleanup completed",
        "Do not target a fixed candidate count",
        "Use the shortest contiguous quote that establishes user authority for the complete candidate; never paraphrase it",
        "An imperative request such as \"Remember that.\" or \"Remember this.\" MUST produce a candidate",
        "Do not extract vague standalone references",
        "Return candidates in source-turn order",
    ] {
        assert!(
            INSOMNIA_SYSTEM_PROMPT.contains(clause),
            "missing legacy extraction-contract clause: {clause}"
        );
    }
    assert!(INSOMNIA_SYSTEM_PROMPT.contains("Do you remember that?"));
    assert!(INSOMNIA_SYSTEM_PROMPT.contains("MUST NOT be treated as adoption"));
    assert!(INSOMNIA_SYSTEM_PROMPT.contains("authority_kind must be exactly one of"));
    assert!(
        INSOMNIA_SYSTEM_PROMPT
            .contains("A question must never be converted into an asserted decision")
    );
    assert!(INSOMNIA_SYSTEM_PROMPT.contains("A vague or deictic user turn"));
}

#[test]
fn authority_kind_is_required_and_preserved_by_structured_extraction() {
    let schema = crate::insomnia_schema();
    let candidate_schema = &schema["properties"]["candidates"]["items"];
    assert_eq!(
        candidate_schema["properties"]["authority_kind"]["enum"],
        json!(["direct", "correction", "adoption", "retention"])
    );
    assert!(
        candidate_schema["required"]
            .as_array()
            .unwrap()
            .iter()
            .any(|field| field == "authority_kind")
    );

    let path = test_path("authority-kind.cva");
    let mut cva = Cva::create(&path).unwrap();
    append(
        &mut cva,
        "u0",
        None,
        "user",
        10,
        "I prefer concise answers.",
    );
    let episode = queue_episode(&mut cva, "u0");
    let turns = cva.episode_turns(episode.id).unwrap();
    let extractor = InsomniaExtractor::new(SimulatedGeneralEndpoint::new(
        "test-model",
        vec![json!({
            "candidates": [{
                "authority_kind": "direct",
                "category": "preference",
                "type": "communication",
                "title": "Concise responses",
                "content": "The user prefers concise answers.",
                "source_node_id": "u0",
                "source_quote": "I prefer concise answers.",
                "content_source_conversation_id": "",
                "content_source_node_id": "",
                "content_source_quote": ""
            }],
            "evidence_requests": []
        })],
    ));
    let extraction = extractor.extract(&episode, &turns).unwrap();
    assert_eq!(extraction.candidates.len(), 1);
    assert_eq!(extraction.candidates[0].authority_kind, "direct");
}

#[test]
fn deterministic_authority_policy_rejects_missing_adoption_provenance() {
    let path = test_path("adoption-without-provenance.cva");
    let mut cva = Cva::create(&path).unwrap();
    append(&mut cva, "u0", None, "user", 10, "alright, that works");
    let episode = queue_episode(&mut cva, "u0");
    let turns = cva.episode_turns(episode.id).unwrap();
    let extractor = InsomniaExtractor::new(SimulatedGeneralEndpoint::new(
        "test-model",
        vec![json!({
            "candidates": [{
                "authority_kind": "adoption",
                "category": "decision",
                "type": "project",
                "title": "Server state response",
                "content": "The server returns the updated Player state.",
                "source_node_id": "u0",
                "source_quote": "alright, that works",
                "content_source_conversation_id": "",
                "content_source_node_id": "",
                "content_source_quote": ""
            }],
            "evidence_requests": []
        })],
    ));
    let extraction = extractor.extract(&episode, &turns).unwrap();
    assert!(extraction.candidates.is_empty());
    assert_eq!(extraction.rejected.len(), 1);
    assert!(
        extraction.rejected[0]
            .reason
            .contains("requires assistant content provenance")
    );
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
