use crate::{Cva, FragmentConfig};
use std::fs;
use uuid::Uuid;

#[test]
fn archive_search_returns_hydrated_historical_fragments() {
    let dir = std::env::temp_dir().join(format!("reliquary-archive-search-{}", Uuid::new_v4()));
    fs::create_dir_all(&dir).unwrap();
    let path = dir.join("project.rel");
    let mut rel = Cva::create_project(&path).unwrap();
    rel.append_node(
        "u1".into(),
        "history".into(),
        None,
        "user".into(),
        1,
        "historical alphaarchivetoken detail",
    )
    .unwrap();
    rel.append_node(
        "a1".into(),
        "history".into(),
        Some("u1".into()),
        "assistant".into(),
        2,
        "ack",
    )
    .unwrap();
    rel.materialize_path_fragments("history", "a1", FragmentConfig::default(), true)
        .unwrap();

    let hits = rel.search_archive("alphaarchivetoken", 5).unwrap();
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].fragment.conversation_id, "history");
    assert!(hits[0].text.contains("alphaarchivetoken"));
}
