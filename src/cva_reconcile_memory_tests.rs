use crate::{
    Cva, CvaReconcileConflict, CvaReconcileError, EpisodeBoundary, EpisodeConfig, EpisodeOrigin,
    InsomniaPriority, InsomniaWorkState, MemoryDraft, WorkspaceMetadata,
};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

fn test_dir() -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("continuity-cva-memory-merge-{unique}"));
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn create_workspace(path: &Path) {
    let metadata = WorkspaceMetadata::new("workspace-1", "Project", "construction").unwrap();
    Cva::create_workspace(path, metadata)
        .unwrap()
        .sync()
        .unwrap();
}

fn append(cva: &mut Cva, id: &str, conversation: &str, content: &str) {
    cva.append_node(
        id.into(),
        conversation.into(),
        None,
        "user".into(),
        1,
        content,
    )
    .unwrap();
}

fn seed_episode(cva: &mut Cva, conversation: &str) -> crate::Episode {
    append(cva, "u", conversation, "remember this");
    cva.append_node(
        "a".into(),
        conversation.into(),
        Some("u".into()),
        "assistant".into(),
        2,
        "noted",
    )
    .unwrap();
    cva.materialize_path_episodes(
        conversation,
        "a",
        EpisodeConfig::default(),
        EpisodeOrigin::Live,
        Some((EpisodeBoundary::Inactivity, 3)),
    )
    .unwrap()
    .created[0]
        .clone()
}

fn draft(episode: &crate::Episode, mutation: &str, state: &str) -> MemoryDraft {
    MemoryDraft {
        category: "decision".into(),
        memory_type: "project".into(),
        authority_kind: "unknown".into(),
        title: "Decision".into(),
        content: "Keep this".into(),
        scope: "private".into(),
        lifecycle_state: state.into(),
        archived: false,
        superseded_by: None,
        parent_id: None,
        source_node_id: Some("u".into()),
        content_source_conversation_id: None,
        content_source_node_id: None,
        grounding_source_conversation_id: None,
        grounding_source_node_id: None,
        source_episode_id: Some(episode.id),
        mutation_id: mutation.into(),
        created_at_ns: 3,
        updated_at_ns: 4,
    }
}

#[test]
fn reconcile_replays_memory_episode_and_completion_together() {
    let dir = test_dir();
    let left = dir.join("left.cva");
    let right = dir.join("right.cva");
    let output = dir.join("merged.cva");
    create_workspace(&left);
    fs::copy(&left, &right).unwrap();
    let mut left_cva = Cva::open(&left).unwrap();
    append(&mut left_cva, "left", "left-c", "left");
    left_cva.sync().unwrap();

    let mut right_cva = Cva::open(&right).unwrap();
    let episode = seed_episode(&mut right_cva, "c");
    let (memory, _) = right_cva
        .publish_memory(None, 0, draft(&episode, "right-memory", "extracted"))
        .unwrap();
    right_cva
        .queue_insomnia_episode(episode.id, InsomniaPriority::Live, 5)
        .unwrap();
    let claim = right_cva
        .claim_insomnia_episode("worker", 6, 100)
        .unwrap()
        .unwrap();
    right_cva
        .complete_insomnia_episode(
            episode.id,
            claim.lease_token.unwrap(),
            6,
            7,
            "test-model".into(),
            "v1".into(),
            vec![memory.id],
            0,
        )
        .unwrap();
    right_cva.sync().unwrap();
    drop(right_cva);

    let result = Cva::reconcile(&left, &right, &output).unwrap();
    assert_eq!(result.replayed_memory_revisions, 1);
    assert_eq!(result.replayed_insomnia_completions, 1);
    let mut merged = Cva::open(output).unwrap();
    assert_eq!(
        merged.memory(memory.id).unwrap().mutation_id,
        "right-memory"
    );
    assert!(merged.episode(episode.id).is_some());
    let attempts = merged.insomnia_attempts(episode.id);
    assert_eq!(attempts.len(), 1);
    assert_eq!(attempts[0].state, InsomniaWorkState::Complete);
    assert_eq!(attempts[0].memory_ids, vec![memory.id]);
}

#[test]
fn reconcile_surfaces_conflicting_memory_revisions() {
    let dir = test_dir();
    let left = dir.join("left.cva");
    let right = dir.join("right.cva");
    let output = dir.join("merged.cva");
    create_workspace(&left);
    let mut base = Cva::open(&left).unwrap();
    let episode = seed_episode(&mut base, "c");
    let (first, _) = base
        .publish_memory(None, 0, draft(&episode, "first", "extracted"))
        .unwrap();
    base.sync().unwrap();
    drop(base);
    fs::copy(&left, &right).unwrap();

    for (path, mutation, state) in [
        (&left, "left-revision", "reinforced"),
        (&right, "right-revision", "canonical"),
    ] {
        let mut cva = Cva::open(path).unwrap();
        cva.publish_memory(Some(first.id), 1, draft(&episode, mutation, state))
            .unwrap();
        cva.sync().unwrap();
    }

    assert!(matches!(
        Cva::reconcile(&left, &right, &output),
        Err(CvaReconcileError::Conflict(
            CvaReconcileConflict::MemoryRevision {
                memory_id,
                expected_revision: 1,
                current_revision: 2,
                incoming_revision: 2,
            }
        )) if memory_id == first.id
    ));
    assert!(!output.exists());
}
