use crate::{Cva, CvaReconcileConflict, CvaReconcileError, MemoryDraft};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

fn test_dir() -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("continuity-cva-conflicts-{unique}"));
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn create_workspace(path: &Path) {
    Cva::create_project(path).unwrap().sync().unwrap();
}

fn append(path: &Path, content: &str) {
    let mut cva = Cva::open(path).unwrap();
    cva.append_node("same".into(), "c".into(), None, "user".into(), 1, content)
        .unwrap();
    cva.sync().unwrap();
}

#[test]
fn source_turn_conflict_exposes_stable_kind_and_identity() {
    let dir = test_dir();
    let left = dir.join("left.cva");
    let right = dir.join("right.cva");
    let output = dir.join("merged.cva");
    create_workspace(&left);
    fs::copy(&left, &right).unwrap();
    append(&left, "left");
    append(&right, "right");

    let error = Cva::reconcile(&left, &right, &output).unwrap_err();
    let conflict = error.conflict().expect("typed reconciliation conflict");
    assert_eq!(conflict.kind(), "source_turn");
    assert_eq!(
        conflict,
        &CvaReconcileConflict::SourceTurn {
            conversation_id: "c".into(),
            node_id: "same".into(),
        }
    );
    assert!(matches!(error, CvaReconcileError::Conflict(_)));
    assert!(!output.exists());
}

fn memory_draft(content: &str) -> MemoryDraft {
    MemoryDraft {
        category: "fact".into(),
        memory_type: "project".into(),
        authority_kind: "unknown".into(),
        title: "Shared mutation".into(),
        content: content.into(),
        scope: "private".into(),
        lifecycle_state: "extracted".into(),
        archived: false,
        superseded_by: None,
        parent_id: None,
        source_node_id: None,
        content_source_conversation_id: None,
        content_source_node_id: None,
        grounding_source_conversation_id: None,
        grounding_source_node_id: None,
        source_episode_id: None,
        mutation_id: "same-mutation".into(),
        created_at_ns: 1,
        updated_at_ns: 1,
    }
}

#[test]
fn memory_mutation_conflict_exposes_mutation_identity() {
    let dir = test_dir();
    let left = dir.join("left.cva");
    let right = dir.join("right.cva");
    let output = dir.join("merged.cva");
    create_workspace(&left);
    fs::copy(&left, &right).unwrap();

    let left_memory = {
        let mut cva = Cva::open(&left).unwrap();
        let (memory, _) = cva.publish_memory(None, 0, memory_draft("left")).unwrap();
        cva.sync().unwrap();
        memory
    };
    {
        let mut cva = Cva::open(&right).unwrap();
        cva.publish_memory(None, 0, memory_draft("right")).unwrap();
        cva.sync().unwrap();
    }

    let error = Cva::reconcile(&left, &right, &output).unwrap_err();
    assert!(matches!(
        error.conflict(),
        Some(CvaReconcileConflict::MemoryMutation {
            memory_id,
            mutation_id,
            incoming_revision: 1,
        }) if *memory_id == left_memory.id && mutation_id == "same-mutation"
    ));
    assert!(!output.exists());
}
