use crate::{
    Cva, InteractionAttachment, InteractionRole, InteractionRuntime, InteractionTurn,
    InteractionTurnStatus,
};
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

fn test_path() -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("continuity-interaction-runtime-{unique}"));
    fs::create_dir_all(&dir).unwrap();
    dir.join("archive.cva")
}

fn user_turn() -> InteractionTurn {
    InteractionTurn {
        message_id: "message-1".into(),
        session_id: "session-1".into(),
        parent_message_id: None,
        role: InteractionRole::User,
        timestamp_ns: 1,
        content: "Review the attached plan.".into(),
        attachments: vec![InteractionAttachment {
            filename: "plan.pdf".into(),
            mime_type: Some("application/pdf".into()),
            bytes: b"plan bytes".to_vec(),
        }],
    }
}

#[test]
fn normalized_turn_is_durably_acknowledged() {
    let path = test_path();
    let cva = Cva::create(&path).unwrap();
    let mut runtime = InteractionRuntime::new(cva);

    let receipt = runtime.accept_turn(user_turn()).unwrap();
    assert_eq!(receipt.archive_version, 1);
    assert_eq!(receipt.turn.node.id, "message-1");
    assert_eq!(receipt.turn.node.conversation_id, "session-1");
    assert_eq!(receipt.turn.node.role, "user");
    assert_eq!(receipt.turn.attachments.len(), 1);
    drop(runtime);

    let reopened = Cva::open(path).unwrap();
    assert_eq!(reopened.archive_version(), 1);
    assert_eq!(reopened.stats().nodes, 1);
    assert_eq!(reopened.stats().files, 1);
    assert_eq!(reopened.stats().source_attachments, 1);
    assert_eq!(reopened.files_for_source("session-1", "message-1").len(), 1);
}

#[test]
fn replayed_normalized_turn_is_idempotent() {
    let path = test_path();
    let cva = Cva::create(path).unwrap();
    let mut runtime = InteractionRuntime::new(cva);

    let first = runtime.accept_turn(user_turn()).unwrap();
    let second = runtime.accept_turn(user_turn()).unwrap();

    assert_eq!(second.turn, first.turn);
    assert_eq!(second.archive_version, first.archive_version);
    assert_eq!(runtime.cva().stats().nodes, 1);
    assert_eq!(runtime.cva().stats().files, 1);
}

#[test]
fn normalized_agent_role_maps_to_archive_assistant_role() {
    let path = test_path();
    let cva = Cva::create(path).unwrap();
    let mut runtime = InteractionRuntime::new(cva);
    runtime.accept_turn(user_turn()).unwrap();

    let receipt = runtime
        .accept_turn(InteractionTurn {
            message_id: "message-2".into(),
            session_id: "session-1".into(),
            parent_message_id: Some("message-1".into()),
            role: InteractionRole::Agent,
            timestamp_ns: 2,
            content: "The plan is internally consistent.".into(),
            attachments: Vec::new(),
        })
        .unwrap();

    assert_eq!(receipt.turn.node.role, "assistant");
    assert_eq!(receipt.turn.node.parent_id.as_deref(), Some("message-1"));
    assert_eq!(receipt.archive_version, 2);
}

#[test]
fn checkpointed_stream_survives_reopen_as_interrupted() {
    let path = test_path();
    let cva = Cva::create(&path).unwrap();
    let mut runtime = InteractionRuntime::new(cva);
    runtime.open_session("session-1".into(), None).unwrap();
    runtime
        .begin_message("session-1", "message-1".into(), InteractionRole::User, 1)
        .unwrap();
    runtime
        .append_text("session-1", "message-1", "Start")
        .unwrap();
    runtime.complete_message("session-1", "message-1").unwrap();
    runtime
        .begin_message("session-1", "message-2".into(), InteractionRole::Agent, 2)
        .unwrap();
    runtime
        .append_checkpointed_text("session-1", "message-2", "Partial answer")
        .unwrap();

    let live = runtime
        .conversation_transcript("session-1", "message-1")
        .unwrap();
    assert_eq!(live.len(), 2);
    assert_eq!(live[1].content, "Partial answer");
    assert_eq!(live[1].status, InteractionTurnStatus::Streaming);
    drop(runtime);

    let cva = Cva::open(path).unwrap();
    let mut reopened = InteractionRuntime::new(cva);
    reopened
        .open_session("session-1".into(), Some("message-1".into()))
        .unwrap();
    let recovered = reopened
        .conversation_transcript("session-1", "message-1")
        .unwrap();
    assert_eq!(recovered.len(), 2);
    assert_eq!(recovered[1].content, "Partial answer");
    assert_eq!(recovered[1].status, InteractionTurnStatus::Interrupted);
}

#[test]
fn completed_stream_replaces_checkpoint_without_duplicate_transcript_turn() {
    let path = test_path();
    let cva = Cva::create(path).unwrap();
    let mut runtime = InteractionRuntime::new(cva);
    runtime.open_session("session-1".into(), None).unwrap();
    runtime
        .begin_message("session-1", "message-1".into(), InteractionRole::User, 1)
        .unwrap();
    runtime
        .append_text("session-1", "message-1", "Start")
        .unwrap();
    runtime.complete_message("session-1", "message-1").unwrap();
    runtime
        .begin_message("session-1", "message-2".into(), InteractionRole::Agent, 2)
        .unwrap();
    runtime
        .append_checkpointed_text("session-1", "message-2", "Complete answer")
        .unwrap();
    runtime.complete_message("session-1", "message-2").unwrap();

    let transcript = runtime
        .conversation_transcript("session-1", "message-2")
        .unwrap();
    assert_eq!(transcript.len(), 2);
    assert_eq!(transcript[1].content, "Complete answer");
    assert_eq!(transcript[1].status, InteractionTurnStatus::Complete);
}
