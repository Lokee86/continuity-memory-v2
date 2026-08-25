use crate::{
    Cva, CvaReconcileError, CvaRelation, InteractionRole, InteractionRuntime,
    InteractionStreamStatus, WorkspaceMetadata,
};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

fn test_dir() -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("continuity-cva-reconcile-{unique}"));
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn metadata(id: &str) -> WorkspaceMetadata {
    WorkspaceMetadata::new(id, "Project", "construction").unwrap()
}

fn create_base(path: &Path, id: &str) {
    let cva = Cva::create_workspace(path, metadata(id)).unwrap();
    cva.sync().unwrap();
}

#[test]
fn compare_identical_copies() {
    let dir = test_dir();
    let left = dir.join("left.cva");
    let right = dir.join("right.cva");
    create_base(&left, "workspace-1");
    fs::copy(&left, &right).unwrap();

    let comparison = Cva::compare(&left, &right).unwrap();
    assert_eq!(comparison.relation, CvaRelation::Identical);
    assert_eq!(comparison.left_chunk_count, comparison.right_chunk_count);
    assert_eq!(comparison.common_chunk_count, comparison.left_chunk_count);
}

#[test]
fn compare_detects_one_side_ahead() {
    let dir = test_dir();
    let left = dir.join("left.cva");
    let right = dir.join("right.cva");
    create_base(&left, "workspace-1");
    fs::copy(&left, &right).unwrap();

    let mut left_cva = Cva::open(&left).unwrap();
    left_cva
        .append_node(
            "node-a".into(),
            "conversation-a".into(),
            None,
            "user".into(),
            1,
            "left-only",
        )
        .unwrap();
    left_cva.sync().unwrap();
    drop(left_cva);

    let comparison = Cva::compare(&left, &right).unwrap();
    assert_eq!(comparison.relation, CvaRelation::LeftExtendsRight);
    assert_eq!(comparison.common_chunk_count, comparison.right_chunk_count);
}

#[test]
fn compare_detects_divergent_tails() {
    let dir = test_dir();
    let left = dir.join("left.cva");
    let right = dir.join("right.cva");
    create_base(&left, "workspace-1");
    fs::copy(&left, &right).unwrap();

    for (path, id, content) in [
        (&left, "node-a", "left-only"),
        (&right, "node-b", "right-only"),
    ] {
        let mut cva = Cva::open(path).unwrap();
        cva.append_node(
            id.into(),
            "conversation-a".into(),
            None,
            "user".into(),
            1,
            content,
        )
        .unwrap();
        cva.sync().unwrap();
    }

    let comparison = Cva::compare(&left, &right).unwrap();
    assert_eq!(comparison.relation, CvaRelation::Diverged);
    assert!(comparison.common_chunk_count < comparison.left_chunk_count);
    assert!(comparison.common_chunk_count < comparison.right_chunk_count);
}

#[test]
fn divergent_reconcile_preserves_interrupted_interaction_streams() {
    let dir = test_dir();
    let left = dir.join("left.cva");
    let right = dir.join("right.cva");
    let output = dir.join("merged.cva");
    create_base(&left, "workspace-1");
    fs::copy(&left, &right).unwrap();

    let mut left_cva = Cva::open(&left).unwrap();
    left_cva
        .append_node(
            "left-node".into(),
            "left-conversation".into(),
            None,
            "user".into(),
            1,
            "left change",
        )
        .unwrap();
    left_cva.sync().unwrap();
    drop(left_cva);

    let right_cva = Cva::open(&right).unwrap();
    let mut runtime = InteractionRuntime::new(right_cva);
    runtime.open_session("chat".into(), None).unwrap();
    runtime
        .begin_message("chat", "user-1".into(), InteractionRole::User, 2)
        .unwrap();
    runtime.append_text("chat", "user-1", "Question").unwrap();
    runtime.complete_message("chat", "user-1").unwrap();
    runtime
        .begin_message("chat", "assistant-1".into(), InteractionRole::Agent, 3)
        .unwrap();
    runtime
        .append_checkpointed_text("chat", "assistant-1", "Visible partial")
        .unwrap();
    runtime.interrupt_message("chat", "assistant-1").unwrap();
    drop(runtime);

    let result = Cva::reconcile(&left, &right, &output).unwrap();
    assert!(result.canonical_change_required);
    let merged = Cva::open(output).unwrap();
    let streams = merged.interaction_stream_records();
    assert_eq!(streams.len(), 1);
    assert_eq!(streams[0].content, "Visible partial");
    assert_eq!(streams[0].status, InteractionStreamStatus::Interrupted);
}

#[test]
fn compare_rejects_different_workspaces() {
    let dir = test_dir();
    let left = dir.join("left.cva");
    let right = dir.join("right.cva");
    create_base(&left, "workspace-1");
    create_base(&right, "workspace-2");

    assert!(matches!(
        Cva::compare(&left, &right),
        Err(CvaReconcileError::WorkspaceMismatch { .. })
    ));
}
