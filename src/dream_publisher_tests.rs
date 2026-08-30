use crate::dream_candidate_test_support::{memory, test_path};
use crate::{
    Cva, DreamEvidenceSide, DreamPairClassification, DreamPairEvidence, DreamPairVerification,
    DreamPublicationError, DreamPublicationOutcome, DreamRelationDirection, DreamRelationKind,
    DreamVerificationPolicy, DreamVerificationSignal, DreamVerificationVerdict, GraphRelationKind,
};

fn pair() -> (Cva, crate::MemoryId, crate::MemoryId) {
    let mut cva = Cva::create(test_path("publisher.cva")).unwrap();
    let left = memory(&mut cva, "left", "Left", "Left memory.", 100, false);
    let right = memory(&mut cva, "right", "Right", "Right memory.", 110, false);
    let (a, b) = if left.0 < right.0 {
        (left, right)
    } else {
        (right, left)
    };
    (cva, a, b)
}

fn classification(
    a: crate::MemoryId,
    b: crate::MemoryId,
    relation: DreamRelationKind,
    direction: DreamRelationDirection,
) -> DreamPairClassification {
    DreamPairClassification {
        model: "classifier".into(),
        a,
        b,
        relation,
        direction,
        evidence: if relation == DreamRelationKind::None {
            Vec::new()
        } else {
            vec![
                DreamPairEvidence {
                    side: DreamEvidenceSide::A,
                    quote: "A evidence".into(),
                },
                DreamPairEvidence {
                    side: DreamEvidenceSide::B,
                    quote: "B evidence".into(),
                },
            ]
        },
    }
}

fn verification(
    classification: &DreamPairClassification,
    verdict: DreamVerificationVerdict,
) -> DreamPairVerification {
    let signal = match verdict {
        DreamVerificationVerdict::Accept => DreamVerificationSignal::Yes,
        DreamVerificationVerdict::Reject => DreamVerificationSignal::No,
        DreamVerificationVerdict::Uncertain => DreamVerificationSignal::Uncertain,
    };
    DreamPairVerification {
        verifier_model: "verifier".into(),
        a: classification.a,
        b: classification.b,
        classification: classification.clone(),
        relation_supported: signal,
        direction_supported: DreamVerificationSignal::Yes,
        evidence_supported: DreamVerificationSignal::Yes,
        verdict,
    }
}

#[test]
fn undirected_relation_publishes_reciprocal_edges_in_one_graph_transaction() {
    let (mut cva, a, b) = pair();
    let proposed = classification(
        a,
        b,
        DreamRelationKind::Topical,
        DreamRelationDirection::Undirected,
    );
    let outcome = cva
        .publish_dream_pair(&proposed, None, DreamVerificationPolicy::default(), 0)
        .unwrap();
    let DreamPublicationOutcome::Published(changes) = outcome else {
        panic!("expected publication");
    };
    assert_eq!(changes.len(), 2);
    assert!(changes.iter().all(|change| change.graph_version == 1));
    assert_eq!(changes[0].global_version, changes[1].global_version);
    assert_eq!(cva.graph_version(), 1);
    let relations = cva.graph_relations();
    assert!(relations.iter().any(|relation| {
        relation.source == a && relation.target == b && relation.kind == GraphRelationKind::Topical
    }));
    assert!(relations.iter().any(|relation| {
        relation.source == b && relation.target == a && relation.kind == GraphRelationKind::Topical
    }));
}

#[test]
fn later_positive_relation_adds_without_retracting_existing_relation() {
    let (mut cva, a, b) = pair();
    let topical = classification(
        a,
        b,
        DreamRelationKind::Topical,
        DreamRelationDirection::Undirected,
    );
    cva.publish_dream_pair(&topical, None, DreamVerificationPolicy::default(), 0)
        .unwrap();

    let factual = classification(
        a,
        b,
        DreamRelationKind::Factual,
        DreamRelationDirection::BToA,
    );
    let outcome = cva
        .publish_dream_pair(&factual, None, DreamVerificationPolicy::default(), 1)
        .unwrap();
    let DreamPublicationOutcome::Published(changes) = outcome else {
        panic!("expected additive publication");
    };
    assert_eq!(changes.len(), 1);
    assert_eq!(changes[0].graph_version, 2);
    assert_eq!(cva.graph_version(), 2);
    assert_eq!(cva.graph_stats().relation_mutations, 3);
    let relations = cva.graph_relations();
    assert_eq!(relations.len(), 3);
    assert!(relations.iter().any(|relation| {
        relation.source == a && relation.target == b && relation.kind == GraphRelationKind::Topical
    }));
    assert!(relations.iter().any(|relation| {
        relation.source == b && relation.target == a && relation.kind == GraphRelationKind::Topical
    }));
    assert!(relations.iter().any(|relation| {
        relation.source == b && relation.target == a && relation.kind == GraphRelationKind::Factual
    }));
}

#[test]
fn none_is_a_no_op_and_cannot_retract_existing_relations() {
    let (mut cva, a, b) = pair();
    cva.set_memory_relation(a, b, GraphRelationKind::StructuralParent, true, 0)
        .unwrap();
    let causal = classification(
        a,
        b,
        DreamRelationKind::Causal,
        DreamRelationDirection::AToB,
    );
    cva.publish_dream_pair(&causal, None, DreamVerificationPolicy::default(), 1)
        .unwrap();
    let none = classification(a, b, DreamRelationKind::None, DreamRelationDirection::None);
    assert_eq!(
        cva.publish_dream_pair(&none, None, DreamVerificationPolicy::default(), 2)
            .unwrap(),
        DreamPublicationOutcome::NoChange
    );

    assert_eq!(cva.graph_version(), 2);
    let relations = cva.graph_relations();
    assert_eq!(relations.len(), 2);
    assert!(relations.iter().any(|relation| {
        relation.source == a
            && relation.target == b
            && relation.kind == GraphRelationKind::StructuralParent
    }));
    assert!(relations.iter().any(|relation| {
        relation.source == a && relation.target == b && relation.kind == GraphRelationKind::Causal
    }));
}

#[test]
fn required_verification_must_accept_before_publication() {
    let (mut cva, a, b) = pair();
    let proposed = classification(
        a,
        b,
        DreamRelationKind::Supersedes,
        DreamRelationDirection::AToB,
    );
    assert!(matches!(
        cva.publish_dream_pair(&proposed, None, DreamVerificationPolicy::default(), 0),
        Err(DreamPublicationError::MissingVerification)
    ));
    let rejected = verification(&proposed, DreamVerificationVerdict::Reject);
    assert_eq!(
        cva.publish_dream_pair(
            &proposed,
            Some(&rejected),
            DreamVerificationPolicy::default(),
            0,
        )
        .unwrap(),
        DreamPublicationOutcome::Withheld(DreamVerificationVerdict::Reject)
    );
    assert_eq!(cva.graph_version(), 0);

    let accepted = verification(&proposed, DreamVerificationVerdict::Accept);
    assert!(matches!(
        cva.publish_dream_pair(
            &proposed,
            Some(&accepted),
            DreamVerificationPolicy::default(),
            0,
        )
        .unwrap(),
        DreamPublicationOutcome::Published(_)
    ));
    assert_eq!(cva.graph_version(), 1);
}

#[test]
fn verified_duplicate_publishes_one_chronological_predecessor_edge() {
    let (mut cva, a, b) = pair();
    let proposed = classification(
        a,
        b,
        DreamRelationKind::DuplicateOf,
        DreamRelationDirection::Undirected,
    );
    let accepted = verification(&proposed, DreamVerificationVerdict::Accept);
    let outcome = cva
        .publish_dream_pair(
            &proposed,
            Some(&accepted),
            DreamVerificationPolicy::default(),
            0,
        )
        .unwrap();
    assert!(matches!(outcome, DreamPublicationOutcome::Published(_)));
    assert_eq!(cva.graph_version(), 1);
    let duplicate = cva
        .graph_relations()
        .into_iter()
        .find(|relation| relation.kind == GraphRelationKind::DuplicateOf)
        .unwrap();
    assert_eq!(cva.memory(duplicate.source).unwrap().title, "Right");
    assert_eq!(cva.memory(duplicate.target).unwrap().title, "Left");
}

#[test]
fn stale_expected_version_is_rejected_even_when_pair_is_already_current() {
    let (mut cva, a, b) = pair();
    let proposed = classification(
        a,
        b,
        DreamRelationKind::Factual,
        DreamRelationDirection::AToB,
    );
    cva.publish_dream_pair(&proposed, None, DreamVerificationPolicy::default(), 0)
        .unwrap();
    assert!(matches!(
        cva.publish_dream_pair(&proposed, None, DreamVerificationPolicy::default(), 0),
        Err(DreamPublicationError::Graph(
            crate::GraphError::RevisionConflict { .. }
        ))
    ));
}

#[test]
fn idempotent_publication_does_not_advance_graph_version() {
    let (mut cva, a, b) = pair();
    let proposed = classification(
        a,
        b,
        DreamRelationKind::Factual,
        DreamRelationDirection::AToB,
    );
    cva.publish_dream_pair(&proposed, None, DreamVerificationPolicy::default(), 0)
        .unwrap();
    assert_eq!(
        cva.publish_dream_pair(&proposed, None, DreamVerificationPolicy::default(), 1)
            .unwrap(),
        DreamPublicationOutcome::NoChange
    );
    assert_eq!(cva.graph_version(), 1);
}
