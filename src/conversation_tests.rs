use crate::{Cva, IncomingTurn};
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

fn test_path() -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("continuity-conversations-{unique}"));
    fs::create_dir_all(&dir).unwrap();
    dir.join("archive.cva")
}

fn turn(id: &str, parent: Option<&str>, timestamp_ns: i64, content: &str) -> IncomingTurn {
    IncomingTurn {
        id: id.into(),
        conversation_id: "c1".into(),
        parent_id: parent.map(str::to_owned),
        role: "user".into(),
        timestamp_ns,
        content: content.into(),
        attachments: Vec::new(),
    }
}

#[test]
fn conversation_summary_and_path_survive_reopen() {
    let path = test_path();
    let mut cva = Cva::create(&path).unwrap();
    cva.ingest_turn(turn("u1", None, 10, "First")).unwrap();
    cva.ingest_turn(turn("u2", Some("u1"), 20, "Second"))
        .unwrap();
    cva.sync().unwrap();
    drop(cva);

    let mut reopened = Cva::open(path).unwrap();
    let summaries = reopened.conversation_summaries();
    assert_eq!(summaries.len(), 1);
    assert_eq!(summaries[0].conversation_id, "c1");
    assert_eq!(summaries[0].title, None);
    assert_eq!(summaries[0].leaf_node_ids, vec!["u2"]);
    assert_eq!(summaries[0].turn_count, 2);
    assert_eq!(summaries[0].latest_timestamp_ns, 20);

    let turns = reopened.conversation_turns("c1", "u2").unwrap();
    assert_eq!(turns.len(), 2);
    assert_eq!(turns[0].content, "First");
    assert_eq!(turns[1].content, "Second");
}

#[test]
fn conversation_title_requires_existing_conversation() {
    let path = test_path();
    let mut cva = Cva::create(path).unwrap();
    assert!(matches!(
        cva.set_conversation_title("missing", "Title".into()),
        Err(crate::ArchiveError::MissingConversation)
    ));
}

#[test]
fn conversation_title_round_trips_and_is_idempotent() {
    let path = test_path();
    let mut cva = Cva::create(&path).unwrap();
    cva.ingest_turn(turn("u1", None, 10, "First")).unwrap();
    assert!(
        cva.set_conversation_title("c1", "Imported title".into())
            .unwrap()
    );
    assert!(
        !cva.set_conversation_title("c1", "Imported title".into())
            .unwrap()
    );
    cva.sync().unwrap();
    drop(cva);

    let reopened = Cva::open(path).unwrap();
    assert_eq!(
        reopened.conversation_summaries()[0].title.as_deref(),
        Some("Imported title")
    );
    assert_eq!(
        reopened
            .conversation_metadata("c1")
            .and_then(|metadata| metadata.title.as_deref()),
        Some("Imported title")
    );
}

#[test]
fn conversation_summary_exposes_multiple_durable_leaves() {
    let path = test_path();
    let mut cva = Cva::create(path).unwrap();
    cva.ingest_turn(turn("root", None, 10, "Root")).unwrap();
    cva.ingest_turn(turn("older", Some("root"), 20, "Older branch"))
        .unwrap();
    cva.ingest_turn(turn("newer", Some("root"), 30, "Newer branch"))
        .unwrap();

    let summary = cva.conversation_summaries().remove(0);
    assert_eq!(summary.leaf_node_ids, vec!["newer", "older"]);
    assert_eq!(summary.turn_count, 3);
    assert_eq!(
        cva.conversation_turns("c1", "older").unwrap()[1].content,
        "Older branch"
    );
}

#[test]
fn branch_start_markers_follow_every_durable_fork_on_selected_path() {
    let path = test_path();
    let mut cva = Cva::create(path).unwrap();
    cva.ingest_turn(turn("root", None, 10, "Root")).unwrap();
    cva.ingest_turn(turn("left", Some("root"), 20, "Left"))
        .unwrap();
    cva.ingest_turn(turn("right", Some("root"), 30, "Right"))
        .unwrap();
    cva.ingest_turn(turn("right-a", Some("right"), 40, "Right A"))
        .unwrap();
    cva.ingest_turn(turn("right-b", Some("right"), 50, "Right B"))
        .unwrap();

    assert_eq!(
        cva.conversation_branch_start_node_ids("c1", "left")
            .unwrap(),
        vec!["left"]
    );
    assert_eq!(
        cva.conversation_branch_start_node_ids("c1", "right-a")
            .unwrap(),
        vec!["right", "right-a"]
    );
}
