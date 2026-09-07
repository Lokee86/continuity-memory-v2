use crate::{Cva, SearchError};
use std::fs;
use std::path::PathBuf;

fn test_path(name: &str) -> PathBuf {
    let unique = uuid::Uuid::new_v4();
    let dir = std::env::temp_dir().join(format!("reliquary-conversation-search-{unique}"));
    fs::create_dir_all(&dir).unwrap();
    dir.join(name)
}

#[test]
fn branch_search_is_transient_and_excludes_sibling_branch() {
    let mut cva = Cva::create(test_path("branch.cva")).unwrap();
    append(&mut cva, "root", None, "shared anchor");
    append(&mut cva, "left-1", Some("root"), "left ordinary");
    append(
        &mut cva,
        "left-2",
        Some("left-1"),
        "needle on selected branch",
    );
    append(
        &mut cva,
        "right-1",
        Some("root"),
        "needle sibling needle sibling",
    );
    append(
        &mut cva,
        "right-2",
        Some("right-1"),
        "needle sibling needle sibling",
    );

    assert!(cva.fragments().is_empty());
    let archive_version = cva.archive_version();
    let hits = cva
        .search_conversation_branch("conversation", "left-2", "needle", 10)
        .unwrap();

    assert!(!hits.is_empty());
    assert!(hits.iter().all(|hit| !hit.text.contains("sibling")));
    assert!(hits.iter().any(|hit| hit.text.contains("selected branch")));
    assert_eq!(cva.archive_version(), archive_version);
    assert!(cva.fragments().is_empty());
}

#[test]
fn branch_search_includes_fresh_unfragmented_tail() {
    let mut cva = Cva::create(test_path("fresh.cva")).unwrap();
    for index in 0..10 {
        let parent = (index > 0).then(|| format!("n{}", index - 1));
        append(
            &mut cva,
            &format!("n{index}"),
            parent.as_deref(),
            if index == 9 {
                "fresh-tail-marker"
            } else {
                "ordinary"
            },
        );
    }
    assert!(cva.fragments().is_empty());
    let hits = cva
        .search_conversation_branch("conversation", "n9", "fresh-tail-marker", 10)
        .unwrap();
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].fragment.end_node_id, "n9");
}

#[test]
fn branch_search_validates_query_limit_and_leaf() {
    let mut cva = Cva::create(test_path("validation.cva")).unwrap();
    append(&mut cva, "root", None, "alpha");

    assert!(matches!(
        cva.search_conversation_branch("conversation", "root", " ", 10),
        Err(SearchError::EmptyQuery)
    ));
    assert!(matches!(
        cva.search_conversation_branch("conversation", "root", "alpha", 0),
        Err(SearchError::InvalidConfig)
    ));
    assert!(
        cva.search_conversation_branch("conversation", "missing", "alpha", 10)
            .is_err()
    );
}

fn append(cva: &mut Cva, id: &str, parent: Option<&str>, text: &str) {
    cva.append_node(
        id.into(),
        "conversation".into(),
        parent.map(str::to_owned),
        "user".into(),
        cva.archive_version() as i64 + 1,
        text,
    )
    .unwrap();
}
