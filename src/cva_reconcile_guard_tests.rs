use crate::{
    Cva, CvaReconcileError, EpisodeBoundary, EpisodeConfig, EpisodeOrigin, InsomniaPriority,
    MemoryDraft, WorkspaceMetadata,
};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

fn test_dir() -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("continuity-cva-guard-{unique}"));
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

fn seed_episode(cva: &mut Cva) -> crate::Episode {
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
    cva.materialize_path_episodes(
        "c",
        "a",
        EpisodeConfig::default(),
        EpisodeOrigin::Live,
        Some((EpisodeBoundary::Inactivity, 3)),
    )
    .unwrap()
    .created[0]
        .clone()
}

fn diverge_left(path: &Path) {
    let mut cva = Cva::open(path).unwrap();
    cva.append_node(
        "left".into(),
        "other".into(),
        None,
        "user".into(),
        4,
        "left",
    )
    .unwrap();
    cva.sync().unwrap();
}

#[test]
fn reconcile_refuses_divergent_memories_until_memory_replay_exists() {
    let dir = test_dir();
    let left = dir.join("left.cva");
    let right = dir.join("right.cva");
    let output = dir.join("merged.cva");
    create_workspace(&left);
    let mut base = Cva::open(&left).unwrap();
    let episode = seed_episode(&mut base);
    base.sync().unwrap();
    drop(base);
    fs::copy(&left, &right).unwrap();
    diverge_left(&left);

    let mut right_cva = Cva::open(&right).unwrap();
    right_cva
        .publish_memory(
            None,
            0,
            MemoryDraft {
                category: "decision".into(),
                memory_type: "project".into(),
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
                mutation_id: "merge-memory".into(),
                created_at_ns: 4,
                updated_at_ns: 4,
            },
        )
        .unwrap();
    right_cva.sync().unwrap();
    drop(right_cva);

    assert!(matches!(
        Cva::reconcile(&left, &right, &output),
        Err(CvaReconcileError::UnsupportedSemanticOwner("Memories"))
    ));
    assert!(!output.exists());
}

#[test]
fn reconcile_refuses_divergent_insomnia_completion() {
    let dir = test_dir();
    let left = dir.join("left.cva");
    let right = dir.join("right.cva");
    let output = dir.join("merged.cva");
    create_workspace(&left);
    let mut base = Cva::open(&left).unwrap();
    let episode = seed_episode(&mut base);
    base.queue_insomnia_episode(episode.id, InsomniaPriority::Live, 4)
        .unwrap();
    base.sync().unwrap();
    drop(base);
    fs::copy(&left, &right).unwrap();
    diverge_left(&left);

    let mut right_cva = Cva::open(&right).unwrap();
    let claimed = right_cva
        .claim_insomnia_episode("worker", 5, 100)
        .unwrap()
        .unwrap();
    right_cva
        .complete_insomnia_episode(
            episode.id,
            claimed.lease_token.unwrap(),
            5,
            6,
            "test".into(),
            "v1".into(),
            Vec::new(),
            0,
        )
        .unwrap();
    right_cva.sync().unwrap();
    drop(right_cva);

    assert!(matches!(
        Cva::reconcile(&left, &right, &output),
        Err(CvaReconcileError::UnsupportedSemanticOwner(
            "Insomnia completions"
        ))
    ));
    assert!(!output.exists());
}
