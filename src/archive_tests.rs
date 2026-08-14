use crate::{Archive, Branch};
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
    let mut archive = Archive::create(&path).unwrap();
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

    let mut reopened = Archive::open(&path).unwrap();
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
    let mut archive = Archive::create(path).unwrap();
    archive
        .append_node("n1".into(), "c1".into(), None, "user".into(), 1, "hello")
        .unwrap();
    archive
        .append_node("n1".into(), "c1".into(), None, "user".into(), 1, "hello")
        .unwrap();
    assert_eq!(archive.stats().nodes, 1);
    assert_eq!(archive.stats().content_objects, 1);
}
