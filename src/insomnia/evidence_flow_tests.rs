use crate::{
    Cva, EpisodeBoundary, EpisodeConfig, EpisodeOrigin, FragmentConfig, InsomniaExtractor,
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
    let dir = std::env::temp_dir().join(format!("continuity-insomnia-evidence-flow-{unique}"));
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
fn historical_archive_search_can_supply_adopted_assistant_content() {
    let path = test_path("historical-evidence.cva");
    let mut cva = Cva::create(&path).unwrap();
    cva.append_node(
        "old-u".into(),
        "old".into(),
        None,
        "user".into(),
        1,
        "What are our deployment targets?",
    )
    .unwrap();
    cva.append_node(
        "old-a".into(),
        "old".into(),
        Some("old-u".into()),
        "assistant".into(),
        2,
        "The deployment target is Windows and Linux.",
    )
    .unwrap();
    cva.materialize_path_fragments("old", "old-a", FragmentConfig::default(), true)
        .unwrap();
    append(
        &mut cva,
        "u0",
        None,
        "user",
        10,
        "Remember the deployment target we discussed.",
    );
    append(&mut cva, "a0", Some("u0"), "assistant", 11, "Okay.");
    let episode = queue_episode(&mut cva, "a0");
    let claim = cva.claim_insomnia_episode("w", 110, 100).unwrap().unwrap();
    let extractor = InsomniaExtractor::new(SimulatedGeneralEndpoint::new(
        "test-model",
        vec![
            json!({"candidates": [], "evidence_requests": [{
                "kind": "archive_search", "conversation_id": "old", "node_id": "",
                "start_node_id": "", "end_node_id": "", "query": "deployment target Windows Linux", "limit": 1
            }]}),
            json!({"candidates": [{
                "authority_kind": "retention", "category": "fact", "type": "project", "title": "Deployment targets",
                "content": "The deployment target is Windows and Linux.",
                "source_node_id": "u0", "source_quote": "Remember the deployment target we discussed.",
                "authority_source_conversation_id": "old", "authority_source_node_id": "old-a",
                "authority_source_quote": "The deployment target is Windows and Linux.",
                "grounding_source_conversation_id": "",
                "grounding_source_node_id": "",
                "grounding_source_quote": ""
            }], "evidence_requests": []}),
        ],
    ));
    let result = cva
        .process_claimed_insomnia_episode(&claim, &extractor, "private", 110, 120)
        .unwrap();
    assert_eq!(result.created.len(), 1);
    assert_eq!(
        result.created[0].content_source_conversation_id.as_deref(),
        Some("old")
    );
    assert_eq!(
        result.created[0].content_source_node_id.as_deref(),
        Some("old-a")
    );
    assert_eq!(result.created[0].source_episode_id, Some(episode.id));
}

#[test]
fn external_authority_source_must_have_been_returned_as_evidence() {
    let path = test_path("unsupplied-content-source.cva");
    let mut cva = Cva::create(&path).unwrap();
    cva.append_node(
        "old-u".into(),
        "old".into(),
        None,
        "user".into(),
        1,
        "What are our deployment targets?",
    )
    .unwrap();
    cva.append_node(
        "old-a".into(),
        "old".into(),
        Some("old-u".into()),
        "assistant".into(),
        2,
        "The deployment target is Windows and Linux.",
    )
    .unwrap();
    append(
        &mut cva,
        "u0",
        None,
        "user",
        10,
        "Remember the deployment target we discussed.",
    );
    let episode = queue_episode(&mut cva, "u0");
    let claim = cva.claim_insomnia_episode("w", 110, 100).unwrap().unwrap();
    let extractor = InsomniaExtractor::new(SimulatedGeneralEndpoint::new(
        "test-model",
        vec![json!({"candidates": [{
            "authority_kind": "retention", "category": "fact", "type": "project", "title": "Deployment targets",
            "content": "The deployment target is Windows and Linux.",
            "source_node_id": "u0", "source_quote": "Remember the deployment target we discussed.",
            "authority_source_conversation_id": "old", "authority_source_node_id": "old-a",
            "authority_source_quote": "The deployment target is Windows and Linux.",
            "grounding_source_conversation_id": "",
            "grounding_source_node_id": "",
            "grounding_source_quote": ""
        }], "evidence_requests": []})],
    ));
    let result = cva
        .process_claimed_insomnia_episode(&claim, &extractor, "private", 110, 120)
        .unwrap();
    assert!(result.created.is_empty());
    assert_eq!(result.rejected.len(), 1);
    assert!(
        result.rejected[0]
            .reason
            .contains("was not supplied by bounded archive evidence")
    );
    assert_eq!(cva.memory_stats().memories, 0);
    assert_eq!(
        cva.insomnia_work(episode.id).unwrap().state,
        InsomniaWorkState::Complete
    );
}

#[test]
fn second_archive_evidence_round_is_rejected() {
    let path = test_path("second-evidence-round.cva");
    let mut cva = Cva::create(&path).unwrap();
    append(&mut cva, "u0", None, "user", 10, "Remember that.");
    let episode = queue_episode(&mut cva, "u0");
    let turns = cva.episode_turns(episode.id).unwrap();
    let response = json!({"candidates": [], "evidence_requests": [{
        "kind": "turn", "conversation_id": "c1", "node_id": "u0",
        "start_node_id": "", "end_node_id": "", "query": "", "limit": 0
    }]});
    let extractor = InsomniaExtractor::new(SimulatedGeneralEndpoint::new(
        "test-model",
        vec![response.clone(), response],
    ));
    let error = extractor
        .extract_with_evidence(&mut cva, &episode, &turns)
        .unwrap_err();
    assert!(
        error
            .to_string()
            .contains("more than one archive-evidence round")
    );
}
