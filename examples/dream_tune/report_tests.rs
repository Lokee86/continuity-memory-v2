use super::report::pair_candidate_id;
use continuity_memory::{
    DreamPairClassification, DreamRelationDirection, DreamRelationKind, MemoryId,
};

fn id(byte: u8) -> MemoryId {
    MemoryId([byte; 32])
}

fn classification(a: MemoryId, b: MemoryId) -> DreamPairClassification {
    DreamPairClassification {
        model: "test".into(),
        a,
        b,
        relation: DreamRelationKind::Topical,
        direction: DreamRelationDirection::Undirected,
        evidence: Vec::new(),
    }
}

#[test]
fn candidate_id_is_non_source_side_when_source_is_canonical_a() {
    let source = id(1);
    let candidate = id(9);
    assert_eq!(
        pair_candidate_id(source, &classification(source, candidate)),
        candidate
    );
}

#[test]
fn candidate_id_is_non_source_side_when_source_is_canonical_b() {
    let candidate = id(1);
    let source = id(9);
    assert_eq!(
        pair_candidate_id(source, &classification(candidate, source)),
        candidate
    );
}
