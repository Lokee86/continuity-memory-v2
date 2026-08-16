use crate::{
    Cva, EpisodeBoundary, EpisodeConfig, EpisodeOrigin, InsomniaExtractor, InsomniaPriority,
    SimulatedGeneralEndpoint,
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
    let dir = std::env::temp_dir().join(format!("continuity-insomnia-grounding-{unique}"));
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

fn queue_episode(cva: &mut Cva, leaf: &str) {
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
}

#[test]
fn correction_can_use_grounding_without_adopting_assistant_authority() {
    let path = test_path("grounding.cva");
    let mut cva = Cva::create(&path).unwrap();
    append(
        &mut cva,
        "u0",
        None,
        "user",
        10,
        "Should we add ShipStats now?",
    );
    append(
        &mut cva,
        "a0",
        Some("u0"),
        "assistant",
        20,
        "Step 3 means introduce ShipStats now; I recommend deferring it.",
    );
    append(
        &mut cva,
        "u1",
        Some("a0"),
        "user",
        30,
        "ok, so we do want to add it now",
    );
    queue_episode(&mut cva, "u1");
    let claim = cva.claim_insomnia_episode("w", 110, 100).unwrap().unwrap();
    let extractor = InsomniaExtractor::new(SimulatedGeneralEndpoint::new(
        "test-model",
        vec![json!({
            "candidates": [{
                "authority_kind":"correction", "category":"decision", "type":"project",
                "title":"Add ShipStats now", "content":"Add ShipStats now.",
                "source_node_id":"u1", "source_quote":"we do want to add it now",
                "authority_source_conversation_id":"", "authority_source_node_id":"",
                "authority_source_quote":"", "grounding_source_conversation_id":"c1",
                "grounding_source_node_id":"a0",
                "grounding_source_quote":"Step 3 means introduce ShipStats now"
            }],
            "evidence_requests": []
        })],
    ));
    let result = cva
        .process_claimed_insomnia_episode(&claim, &extractor, "private", 110, 120)
        .unwrap();
    assert_eq!(result.created.len(), 1);
    let memory = &result.created[0];
    assert!(memory.content_source_node_id.is_none());
    assert_eq!(memory.grounding_source_node_id.as_deref(), Some("a0"));
    assert_eq!(memory.content, "Add ShipStats now.");
    let memory_id = memory.id;
    cva.sync().unwrap();
    drop(cva);
    let mut reopened = Cva::open(&path).unwrap();
    let memory = reopened.memory(memory_id).unwrap();
    assert!(memory.content_source_node_id.is_none());
    assert_eq!(memory.grounding_source_node_id.as_deref(), Some("a0"));
}
