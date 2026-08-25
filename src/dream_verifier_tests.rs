use crate::dream_candidate_test_support::{install_vectors, memory, test_path};
use crate::{
    DreamCandidateConfig, DreamEvidenceSide, DreamPairClassification, DreamPairEvidence,
    DreamRelationDirection, DreamRelationKind, DreamVerificationError, DreamVerificationPolicy,
    DreamVerificationVerdict, DreamVerifier, GeneralEndpoint, GeneralEndpointError,
};
use serde_json::{Value, json};
use std::sync::{Arc, Mutex};

struct RecordingEndpoint {
    calls: Arc<Mutex<Vec<(String, String, String, Value)>>>,
    response: Value,
}

impl RecordingEndpoint {
    fn new(calls: Arc<Mutex<Vec<(String, String, String, Value)>>>, response: Value) -> Self {
        Self { calls, response }
    }
}

impl GeneralEndpoint for RecordingEndpoint {
    fn model(&self) -> &str {
        "verifier-model"
    }

    fn complete_json(
        &self,
        system_prompt: &str,
        user_payload: &str,
        schema_name: &str,
        schema: &Value,
    ) -> Result<Value, GeneralEndpointError> {
        self.calls.lock().unwrap().push((
            system_prompt.into(),
            user_payload.into(),
            schema_name.into(),
            schema.clone(),
        ));
        Ok(self.response.clone())
    }
}

fn contexts() -> (crate::DreamMemoryContext, crate::DreamMemoryContext) {
    let mut cva = crate::Cva::create(test_path("verifier-pair.cva")).unwrap();
    let first = memory(
        &mut cva,
        "first",
        "Wall size",
        "Exterior walls use 2x6 studs.",
        100,
        false,
    );
    let second = memory(
        &mut cva,
        "second",
        "Wall correction",
        "Correction: exterior walls use 2x8 studs.",
        200,
        false,
    );
    let profile = install_vectors(&mut cva, &[first, second], &[&[1.0, 0.0], &[0.99, 0.01]]);
    let set = cva
        .dream_candidates(
            profile,
            first,
            DreamCandidateConfig {
                limit: 1,
                semantic_limit: 1,
                prior_semantic_quota: 0,
                lexical_limit: 0,
            },
        )
        .unwrap();
    (set.source, set.candidates[0].context.clone())
}

fn classification(
    left: &crate::DreamMemoryContext,
    right: &crate::DreamMemoryContext,
    relation: DreamRelationKind,
    direction: DreamRelationDirection,
) -> DreamPairClassification {
    let (a, b) = if left.memory.id.0 < right.memory.id.0 {
        (left, right)
    } else {
        (right, left)
    };
    DreamPairClassification {
        model: "classifier-model".into(),
        a: a.memory.id,
        b: b.memory.id,
        relation,
        direction,
        evidence: vec![
            DreamPairEvidence {
                side: DreamEvidenceSide::A,
                quote: a.memory.title.clone(),
            },
            DreamPairEvidence {
                side: DreamEvidenceSide::B,
                quote: b.memory.title.clone(),
            },
        ],
    }
}

fn response(relation: &str, direction: &str, evidence: &str) -> Value {
    json!({
        "relation_supported": relation,
        "direction_supported": direction,
        "evidence_supported": evidence,
    })
}

#[test]
fn default_policy_verifies_only_lifecycle_critical_relations() {
    let default = DreamVerificationPolicy::default();
    assert!(!default.should_verify(DreamRelationKind::None));
    assert!(!default.should_verify(DreamRelationKind::Topical));
    assert!(!default.should_verify(DreamRelationKind::Factual));
    assert!(!default.should_verify(DreamRelationKind::Causal));
    assert!(!default.should_verify(DreamRelationKind::Recurrent));
    assert!(default.should_verify(DreamRelationKind::DuplicateOf));
    assert!(default.should_verify(DreamRelationKind::Supersedes));

    let broad = DreamVerificationPolicy::broad_semantic();
    assert!(broad.should_verify(DreamRelationKind::Factual));
    assert!(broad.should_verify(DreamRelationKind::Causal));
    assert!(broad.should_verify(DreamRelationKind::Recurrent));
    assert!(broad.should_verify(DreamRelationKind::DuplicateOf));
    assert!(broad.should_verify(DreamRelationKind::Supersedes));
    assert!(!broad.should_verify(DreamRelationKind::Topical));
}

#[test]
fn verifier_payload_is_identical_for_reverse_processing_direction() {
    let (left, right) = contexts();
    let proposed = classification(
        &left,
        &right,
        DreamRelationKind::Supersedes,
        DreamRelationDirection::BToA,
    );
    let calls = Arc::new(Mutex::new(Vec::new()));
    let verifier = DreamVerifier::new(RecordingEndpoint::new(
        calls.clone(),
        response("yes", "yes", "yes"),
    ));
    let forward = verifier.verify_pair(&proposed, &left, &right).unwrap();
    let reverse = verifier.verify_pair(&proposed, &right, &left).unwrap();

    assert_eq!(forward, reverse);
    assert_eq!(forward.verdict, DreamVerificationVerdict::Accept);
    let calls = calls.lock().unwrap();
    assert_eq!(calls[0].1, calls[1].1);
    assert!(calls[0].0.contains("independent relationship verifier"));
    assert_eq!(calls[0].2, "dream_pair_verification");
    assert_eq!(calls[0].3["additionalProperties"], false);

    let payload: Value = serde_json::from_str(&calls[0].1).unwrap();
    assert_eq!(payload["proposal"]["relation"], "supersedes");
    assert_eq!(payload["proposal"]["direction"], "b_to_a");
    assert_eq!(payload["proposal"]["classifier_model"], "classifier-model");
    assert!(payload.get("confidence").is_none());
}

#[test]
fn deterministic_verdict_uses_three_independent_signals() {
    let (left, right) = contexts();
    let proposed = classification(
        &left,
        &right,
        DreamRelationKind::DuplicateOf,
        DreamRelationDirection::Undirected,
    );

    for (answer, expected) in [
        (
            response("yes", "yes", "yes"),
            DreamVerificationVerdict::Accept,
        ),
        (
            response("yes", "no", "yes"),
            DreamVerificationVerdict::Reject,
        ),
        (
            response("yes", "uncertain", "yes"),
            DreamVerificationVerdict::Uncertain,
        ),
    ] {
        let calls = Arc::new(Mutex::new(Vec::new()));
        let result = DreamVerifier::new(RecordingEndpoint::new(calls, answer))
            .verify_pair(&proposed, &left, &right)
            .unwrap();
        assert_eq!(result.verdict, expected);
    }
}

#[test]
fn policy_skip_makes_no_model_call_and_direct_none_is_rejected() {
    let (left, right) = contexts();
    let factual = classification(
        &left,
        &right,
        DreamRelationKind::Factual,
        DreamRelationDirection::AToB,
    );
    let calls = Arc::new(Mutex::new(Vec::new()));
    let verifier = DreamVerifier::new(RecordingEndpoint::new(
        calls.clone(),
        response("yes", "yes", "yes"),
    ));
    assert!(
        verifier
            .verify_if_required(DreamVerificationPolicy::default(), &factual, &left, &right)
            .unwrap()
            .is_none()
    );
    assert!(calls.lock().unwrap().is_empty());

    let duplicate = classification(
        &left,
        &right,
        DreamRelationKind::DuplicateOf,
        DreamRelationDirection::Undirected,
    );
    let verified = verifier
        .verify_if_required(
            DreamVerificationPolicy::default(),
            &duplicate,
            &left,
            &right,
        )
        .unwrap();
    assert_eq!(verified.unwrap().verdict, DreamVerificationVerdict::Accept);
    assert_eq!(calls.lock().unwrap().len(), 1);

    let mut none = factual;
    none.relation = DreamRelationKind::None;
    none.direction = DreamRelationDirection::None;
    none.evidence.clear();
    assert!(matches!(
        verifier.verify_pair(&none, &left, &right),
        Err(DreamVerificationError::NoRelation)
    ));
}

#[test]
fn mismatched_pair_and_invalid_signal_are_rejected() {
    let (left, right) = contexts();
    let mut proposed = classification(
        &left,
        &right,
        DreamRelationKind::Supersedes,
        DreamRelationDirection::AToB,
    );
    let original_a = proposed.a;
    proposed.a = proposed.b;
    proposed.b = original_a;
    assert!(matches!(
        DreamVerifier::new(RecordingEndpoint::new(
            Arc::new(Mutex::new(Vec::new())),
            response("yes", "yes", "yes"),
        ))
        .verify_pair(&proposed, &left, &right),
        Err(DreamVerificationError::InvalidPair)
    ));

    let valid = classification(
        &left,
        &right,
        DreamRelationKind::Supersedes,
        DreamRelationDirection::AToB,
    );
    assert!(matches!(
        DreamVerifier::new(RecordingEndpoint::new(
            Arc::new(Mutex::new(Vec::new())),
            response("maybe", "yes", "yes"),
        ))
        .verify_pair(&valid, &left, &right),
        Err(DreamVerificationError::InvalidOutput(_))
    ));
}
