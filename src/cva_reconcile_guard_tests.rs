use crate::{
    Cva, CvaReconcileError, EpisodeBoundary, EpisodeConfig, EpisodeOrigin, InsomniaPriority,
    WorkspaceMetadata,
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

fn complete(path: &Path, episode: crate::EpisodeId, model: &str) {
    let mut cva = Cva::open(path).unwrap();
    cva.queue_insomnia_episode(episode, InsomniaPriority::Live, 4)
        .unwrap();
    let claim = cva
        .claim_insomnia_episode("worker", 5, 100)
        .unwrap()
        .unwrap();
    cva.complete_insomnia_episode(
        episode,
        claim.lease_token.unwrap(),
        5,
        6,
        model.into(),
        "v1".into(),
        Vec::new(),
        0,
    )
    .unwrap();
    cva.sync().unwrap();
}

fn append_divergence(path: &Path, id: &str) {
    let mut cva = Cva::open(path).unwrap();
    cva.append_node(id.into(), format!("{id}-c"), None, "user".into(), 7, id)
        .unwrap();
    cva.sync().unwrap();
}

#[test]
fn reconcile_refuses_conflicting_insomnia_completions() {
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

    complete(&left, episode.id, "left-model");
    complete(&right, episode.id, "right-model");

    assert!(matches!(
        Cva::reconcile(&left, &right, &output),
        Err(CvaReconcileError::ConflictingInsomniaCompletion)
    ));
    assert!(!output.exists());
}

#[test]
fn reconcile_deduplicates_identical_completion_on_divergent_copies() {
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

    append_divergence(&left, "left");
    append_divergence(&right, "right");
    complete(&left, episode.id, "same-model");
    complete(&right, episode.id, "same-model");

    let result = Cva::reconcile(&left, &right, &output).unwrap();
    assert_eq!(result.replayed_insomnia_completions, 0);
    let merged = Cva::open(output).unwrap();
    assert_eq!(merged.insomnia_attempts(episode.id).len(), 1);
    assert_eq!(merged.stats().nodes, 4);
}
