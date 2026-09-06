use crate::insomnia::completion::{InsomniaCompletion, InsomniaCompletionBody, encode_completion};
use crate::memory_model::{MemoryRecord, memory_body_bytes, memory_body_id, memory_id};
use crate::{Cva, EpisodeBoundary, EpisodeConfig, EpisodeOrigin, MemoryDraft, MemoryRef};
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

fn test_dir() -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("continuity-cva-grouped-merge-{unique}"));
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn seed_right(cva: &mut Cva) -> (crate::Episode, MemoryDraft) {
    cva.append_node(
        "u".into(),
        "c".into(),
        None,
        "user".into(),
        1,
        "remember this",
    )
    .unwrap();
    cva.append_node(
        "a".into(),
        "c".into(),
        Some("u".into()),
        "assistant".into(),
        2,
        "noted",
    )
    .unwrap();
    let episode = cva
        .materialize_path_episodes(
            "c",
            "a",
            EpisodeConfig::default(),
            EpisodeOrigin::Live,
            Some((EpisodeBoundary::Inactivity, 3)),
        )
        .unwrap()
        .created[0]
        .clone();
    let draft = MemoryDraft {
        category: "decision".into(),
        memory_type: "project".into(),
        authority_kind: "unknown".into(),
        title: "Decision".into(),
        content: "Keep this".into(),
        scope: "private".into(),
        lifecycle_state: "extracted".into(),
        archived: false,
        superseded_by: None,
        parent_id: None,
        source_node_id: Some("u".into()),
        content_source_conversation_id: None,
        content_source_node_id: None,
        grounding_source_conversation_id: None,
        grounding_source_node_id: None,
        source_episode_id: Some(episode.id),
        source_time_ns: None,
        mutation_id: "grouped-memory".into(),
        created_at_ns: 3,
        updated_at_ns: 4,
    };
    (episode, draft)
}

#[test]
fn reconcile_replays_grouped_insomnia_memory_records() {
    let dir = test_dir();
    let left = dir.join("left.cva");
    let right = dir.join("right.cva");
    let output = dir.join("merged.cva");
    Cva::create_project(&left).unwrap().sync().unwrap();
    fs::copy(&left, &right).unwrap();
    let mut left_cva = Cva::open(&left).unwrap();
    left_cva
        .append_node(
            "left".into(),
            "left-c".into(),
            None,
            "user".into(),
            1,
            "left",
        )
        .unwrap();
    left_cva.sync().unwrap();

    let mut right_cva = Cva::open(&right).unwrap();
    let (episode, draft) = seed_right(&mut right_cva);
    let id = memory_id(&draft.mutation_id);
    let body_id = memory_body_id(&draft.title, &draft.content);
    let global_version = right_cva.container.next_version_candidate();
    let record = MemoryRecord {
        id,
        revision: 1,
        body_id,
        category: draft.category,
        memory_type: draft.memory_type,
        authority_kind: draft.authority_kind,
        scope: draft.scope,
        lifecycle_state: draft.lifecycle_state,
        archived: false,
        superseded_by: None,
        parent_id: None,
        source_node_id: draft.source_node_id,
        content_source_conversation_id: None,
        content_source_node_id: None,
        grounding_source_conversation_id: None,
        grounding_source_node_id: None,
        source_episode_id: Some(episode.id),
        source_time_ns: None,
        source_ref: None,
        mutation_id: draft.mutation_id,
        created_at_ns: 3,
        updated_at_ns: 4,
        global_version,
        memory_version: 1,
    };
    let completion = InsomniaCompletion {
        episode_id: episode.id,
        attempt: 1,
        started_at_ns: 5,
        completed_at_ns: 6,
        extractor_model: "test-model".into(),
        extractor_version: "v1".into(),
        rejected_count: 0,
        memory_ids: vec![id],
        external_memory_refs: vec![MemoryRef {
            owner_id: "phy-00000000-0000-0000-0000-000000000001".into(),
            memory_id: id,
        }],
        global_version_start: global_version,
        bodies: vec![InsomniaCompletionBody {
            id: body_id,
            bytes: memory_body_bytes("Decision", "Keep this"),
        }],
        records: vec![record],
    };
    right_cva
        .container
        .append(&encode_completion(&completion).unwrap())
        .unwrap();
    right_cva
        .container
        .commit_embedded_version_range(global_version, 1)
        .unwrap();
    right_cva.sync().unwrap();
    drop(right_cva);
    Cva::open(&right).unwrap();

    let result = Cva::reconcile(&left, &right, &output).unwrap();
    assert_eq!(result.replayed_memory_revisions, 1);
    assert_eq!(result.replayed_insomnia_completions, 1);
    let mut merged = Cva::open(output).unwrap();
    assert_eq!(merged.memory(id).unwrap().mutation_id, "grouped-memory");
    let attempt = &merged.insomnia_attempts(episode.id)[0];
    assert_eq!(attempt.memory_ids, vec![id]);
    assert_eq!(attempt.external_memory_refs[0].memory_id, id);
    assert!(attempt.external_memory_refs[0].owner_id.starts_with("phy-"));
}
