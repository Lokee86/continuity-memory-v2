use crate::lexical_search::lexical_candidates_parts;
use crate::{Cva, FragmentConfig};
use std::fs;
use std::path::PathBuf;

fn test_path(name: &str) -> PathBuf {
    let unique = uuid::Uuid::new_v4();
    let dir = std::env::temp_dir().join(format!("continuity-lexical-index-{unique}"));
    fs::create_dir_all(&dir).unwrap();
    dir.join(name)
}

fn add_fragment(cva: &mut Cva, conversation: &str, text: &str) {
    for index in 0..8 {
        cva.append_node(
            format!("{conversation}-n{index}"),
            conversation.into(),
            (index > 0).then(|| format!("{conversation}-n{}", index - 1)),
            if index % 2 == 0 { "user" } else { "assistant" }.into(),
            index as i64,
            text,
        )
        .unwrap();
    }
    cva.materialize_path_fragments(
        conversation,
        &format!("{conversation}-n7"),
        FragmentConfig::default(),
        false,
    )
    .unwrap();
}

#[test]
fn indexed_lexical_results_match_full_scan_exactly() {
    let path = test_path("equivalence.cva");
    let mut cva = Cva::create(path).unwrap();
    add_fragment(&mut cva, "a", "alpha alphabet alpha beta");
    add_fragment(&mut cva, "b", "alpha beta beta beta");
    add_fragment(&mut cva, "c", "gamma beta delta");

    for query in ["alpha", "beta alpha", "alphabet", "gamma delta", "missing"] {
        let indexed = cva.lexical_candidates(query, 30).unwrap();
        let scanned =
            lexical_candidates_parts(&cva.archive, &mut cva.container, query, 30).unwrap();
        assert_eq!(indexed.len(), scanned.len(), "query={query}");
        for (indexed, scanned) in indexed.iter().zip(&scanned) {
            assert_eq!(indexed.fragment, scanned.fragment, "query={query}");
            assert_eq!(
                indexed.score.to_bits(),
                scanned.score.to_bits(),
                "query={query}"
            );
        }
    }
}

#[test]
fn lexical_index_is_disposable_and_rebuilds_after_reopen() {
    let path = test_path("reopen.cva");
    let mut cva = Cva::create(&path).unwrap();
    add_fragment(&mut cva, "c1", "reopenable lexical state");
    cva.sync().unwrap();
    let before = fs::metadata(&path).unwrap().len();

    assert_eq!(cva.lexical_candidates("reopenable", 10).unwrap().len(), 1);
    cva.sync().unwrap();
    assert_eq!(fs::metadata(&path).unwrap().len(), before);
    drop(cva);

    let mut reopened = Cva::open(&path).unwrap();
    assert_eq!(
        reopened.lexical_candidates("reopenable", 10).unwrap().len(),
        1
    );
    reopened.sync().unwrap();
    assert_eq!(fs::metadata(&path).unwrap().len(), before);
}

#[test]
fn lexical_index_incrementally_adds_new_fragments() {
    let path = test_path("incremental.cva");
    let mut cva = Cva::create(path).unwrap();
    add_fragment(&mut cva, "first", "firstunique durable state");
    assert_eq!(cva.lexical_candidates("firstunique", 10).unwrap().len(), 1);

    add_fragment(&mut cva, "second", "secondunique durable state");
    let second = cva.lexical_candidates("secondunique", 10).unwrap();
    assert_eq!(second.len(), 1);
    assert_eq!(second[0].fragment.conversation_id, "second");

    let both = cva.lexical_candidates("durable state", 10).unwrap();
    assert_eq!(both.len(), 2);
}
