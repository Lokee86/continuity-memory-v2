use crate::archive_codec::encode_node;
use crate::{Branch, Cva, Node};
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

fn test_path() -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("continuity-archive-{unique}"));
    fs::create_dir_all(&dir).unwrap();
    dir.join("archive.cva")
}

#[test]
fn shared_branch_prefix_and_content_are_stored_once() {
    let path = test_path();
    let mut archive = Cva::create(&path).unwrap();
    archive
        .append_node(
            "n1".into(),
            "c1".into(),
            None,
            "user".into(),
            1,
            "same body",
        )
        .unwrap();
    archive
        .append_node(
            "n2".into(),
            "c1".into(),
            Some("n1".into()),
            "assistant".into(),
            2,
            "shared prefix",
        )
        .unwrap();
    archive
        .append_node(
            "a".into(),
            "c1".into(),
            Some("n2".into()),
            "user".into(),
            3,
            "same body",
        )
        .unwrap();
    archive
        .append_node(
            "b".into(),
            "c1".into(),
            Some("n2".into()),
            "user".into(),
            4,
            "other body",
        )
        .unwrap();
    archive
        .append_branch(Branch {
            id: "a".into(),
            conversation_id: "c1".into(),
            leaf_node_id: "a".into(),
            canonical: true,
        })
        .unwrap();
    archive
        .append_branch(Branch {
            id: "b".into(),
            conversation_id: "c1".into(),
            leaf_node_id: "b".into(),
            canonical: false,
        })
        .unwrap();
    assert_eq!(archive.stats().nodes, 4);
    assert_eq!(archive.stats().branches, 2);
    assert_eq!(archive.stats().content_objects, 3);
    archive.sync().unwrap();
    drop(archive);

    let mut reopened = Cva::open(&path).unwrap();
    assert_eq!(reopened.stats().content_objects, 3);
    let a = reopened.branch_turns("c1", "a").unwrap();
    let b = reopened.branch_turns("c1", "b").unwrap();
    assert_eq!(
        a.iter().map(|t| t.node_id.as_str()).collect::<Vec<_>>(),
        ["n1", "n2", "a"]
    );
    assert_eq!(
        b.iter().map(|t| t.node_id.as_str()).collect::<Vec<_>>(),
        ["n1", "n2", "b"]
    );
    assert_eq!(a[0].content, "same body");
    assert_eq!(a[2].content, "same body");
}

#[test]
fn identical_node_append_is_idempotent() {
    let path = test_path();
    let mut archive = Cva::create(path).unwrap();
    archive
        .append_node("n1".into(), "c1".into(), None, "user".into(), 1, "hello")
        .unwrap();
    archive
        .append_node("n1".into(), "c1".into(), None, "user".into(), 1, "hello")
        .unwrap();
    assert_eq!(archive.stats().nodes, 1);
    assert_eq!(archive.stats().content_objects, 1);
}

#[test]
fn unversioned_semantic_record_is_inert_on_reopen() {
    let path = test_path();
    let mut archive = Cva::create(&path).unwrap();
    let first = archive
        .append_node("n1".into(), "c1".into(), None, "user".into(), 1, "hello")
        .unwrap();
    let orphan = Node {
        id: "orphan".into(),
        conversation_id: "c1".into(),
        parent_id: Some("n1".into()),
        role: "assistant".into(),
        timestamp_ns: 2,
        content_id: first.content_id,
    };
    archive
        .container
        .append(&encode_node(&orphan).unwrap())
        .unwrap();
    archive.sync().unwrap();
    drop(archive);

    let reopened = Cva::open(path).unwrap();
    assert_eq!(reopened.stats().nodes, 1);
    assert!(reopened.archive.nodes.get("c1", "orphan").is_none());
}
