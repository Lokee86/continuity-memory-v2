use crate::dream_candidate_test_support::{install_vectors, memory, test_path};
use crate::{
    DreamCandidateConfig, DreamClassificationError, DreamClassifier, DreamEvidenceSide,
    DreamRelationDirection, DreamRelationKind, GeneralEndpoint, GeneralEndpointError,
    SimulatedGeneralEndpoint,
};
use serde_json::{Value, json};
use std::sync::{Arc, Mutex};

struct RecordingEndpoint {
    calls: Arc<Mutex<Vec<(String, String, String, Value)>>>,
}

impl RecordingEndpoint {
    fn new(calls: Arc<Mutex<Vec<(String, String, String, Value)>>>) -> Self {
        Self { calls }
    }
}

impl GeneralEndpoint for RecordingEndpoint {
    fn model(&self) -> &str {
        "recording-dream"
    }

    fn complete_json(
        &self,
        system_prompt: &str,
        user_payload: &str,
        schema_name: &str,
        schema: &Value,
    ) -> Result<Value, GeneralEndpointError> {
        let payload: Value = serde_json::from_str(user_payload)
            .map_err(|error| GeneralEndpointError::Failure(error.to_string()))?;
        self.calls.lock().unwrap().push((
            system_prompt.into(),
            user_payload.into(),
            schema_name.into(),
            schema.clone(),
        ));
        Ok(json!({
            "relation": "topical",
            "direction": "undirected",
            "evidence": [
                {"side": "a", "quote": payload["a"]["title"]},
                {"side": "b", "quote": payload["b"]["title"]}
            ]
        }))
    }
}

fn pair_contexts() -> (crate::DreamMemoryContext, crate::DreamMemoryContext) {
    let mut cva = crate::Cva::create(test_path("classifier-pair.cva")).unwrap();
    let source = memory(
        &mut cva,
        "source",
        "Foundation detail",
        "The foundation uses helical piles.",
        100,
        false,
    );
    let candidate = memory(
        &mut cva,
        "candidate",
        "Pile design",
        "The west wall design relies on the helical piles.",
        110,
        false,
    );
    let profile = install_vectors(
        &mut cva,
        &[source, candidate],
        &[&[1.0, 0.0], &[0.99, 0.01]],
    );
    let set = cva
        .dream_candidates(
            profile,
            source,
            DreamCandidateConfig {
                limit: 1,
                semantic_limit: 1,
                prior_semantic_quota: 0,
                lexical_limit: 0,
                temporal_limit: 0,
            },
        )
        .unwrap();
    (set.source, set.candidates[0].context.clone())
}

#[test]
fn pair_order_is_canonical_and_independent_of_processing_direction() {
    let (left, right) = pair_contexts();
    let calls = Arc::new(Mutex::new(Vec::new()));
    let classifier = DreamClassifier::new(RecordingEndpoint::new(calls.clone()));
    let forward = classifier.classify_pair(&left, &right).unwrap();
    let reverse = classifier.classify_pair(&right, &left).unwrap();

    let calls = calls.lock().unwrap();
    assert_eq!(calls.len(), 2);
    assert_eq!(calls[0].1, calls[1].1);
    assert_eq!(forward, reverse);
    assert!(forward.a.0 < forward.b.0);
    assert_eq!(forward.relation, DreamRelationKind::Topical);
    assert_eq!(forward.direction, DreamRelationDirection::Undirected);
    assert_eq!(forward.evidence.len(), 2);
    assert_eq!(forward.evidence[0].side, DreamEvidenceSide::A);
    assert_eq!(forward.evidence[1].side, DreamEvidenceSide::B);
}

#[test]
fn classifier_uses_strict_pair_schema_and_dream_contract_prompt() {
    let (left, right) = pair_contexts();
    let calls = Arc::new(Mutex::new(Vec::new()));
    let classifier = DreamClassifier::new(RecordingEndpoint::new(calls.clone()));
    classifier.classify_pair(&left, &right).unwrap();

    let calls = calls.lock().unwrap();
    let (prompt, payload, schema_name, schema) = &calls[0];
    assert!(prompt.contains("canonical MemoryId order as A and B"));
    assert_eq!(schema_name, "dream_pair_classification");
    assert_eq!(schema["additionalProperties"], false);
    assert_eq!(schema["properties"]["relation"]["type"], "string");
    assert_eq!(schema["properties"]["evidence"]["maxItems"], 2);
    let payload: Value = serde_json::from_str(payload).unwrap();
    assert!(payload["a"]["temporal"].is_object());
    assert!(payload["b"]["temporal"].is_object());
}

#[test]
fn directional_relation_is_validated_without_using_memory_creation_time() {
    let (left, right) = pair_contexts();
    let (a, b) = if left.memory.id.0 < right.memory.id.0 {
        (&left, &right)
    } else {
        (&right, &left)
    };
    let foundation_is_a = a.memory.title == "Foundation detail";
    let direction = if foundation_is_a { "a_to_b" } else { "b_to_a" };
    let endpoint = SimulatedGeneralEndpoint::new(
        "dream-model",
        vec![json!({
            "relation": "factual",
            "direction": direction,
            "evidence": [
                {"side": "a", "quote": a.memory.title},
                {"side": "b", "quote": b.memory.title}
            ]
        })],
    );
    let result = DreamClassifier::new(endpoint)
        .classify_pair(&left, &right)
        .unwrap();
    assert_eq!(result.relation, DreamRelationKind::Factual);
    assert_eq!(
        result.direction,
        if foundation_is_a {
            DreamRelationDirection::AToB
        } else {
            DreamRelationDirection::BToA
        }
    );
}

#[test]
fn invalid_relation_direction_and_nonverbatim_evidence_are_rejected() {
    let (left, right) = pair_contexts();
    let (a, b) = if left.memory.id.0 < right.memory.id.0 {
        (&left, &right)
    } else {
        (&right, &left)
    };
    let bad_direction = SimulatedGeneralEndpoint::new(
        "dream-model",
        vec![json!({
            "relation": "duplicate_of",
            "direction": "a_to_b",
            "evidence": [
                {"side": "a", "quote": a.memory.title},
                {"side": "b", "quote": b.memory.title}
            ]
        })],
    );
    assert!(matches!(
        DreamClassifier::new(bad_direction).classify_pair(&left, &right),
        Err(DreamClassificationError::InvalidOutput(_))
    ));

    let invented = SimulatedGeneralEndpoint::new(
        "dream-model",
        vec![json!({
            "relation": "topical",
            "direction": "undirected",
            "evidence": [
                {"side": "a", "quote": "invented evidence"},
                {"side": "b", "quote": b.memory.title}
            ]
        })],
    );
    assert!(matches!(
        DreamClassifier::new(invented).classify_pair(&left, &right),
        Err(DreamClassificationError::InvalidOutput(_))
    ));
}

#[test]
fn none_requires_no_evidence_and_candidate_sets_remain_bounded() {
    let mut cva = crate::Cva::create(test_path("classifier-set.cva")).unwrap();
    let source = memory(&mut cva, "source", "Source", "Primary fact.", 100, false);
    let one = memory(&mut cva, "one", "One", "Primary fact one.", 90, false);
    let two = memory(&mut cva, "two", "Two", "Primary fact two.", 80, false);
    let profile = install_vectors(
        &mut cva,
        &[source, one, two],
        &[&[1.0, 0.0], &[0.99, 0.01], &[0.98, 0.02]],
    );
    let set = cva
        .dream_candidates(
            profile,
            source,
            DreamCandidateConfig {
                limit: 2,
                semantic_limit: 2,
                prior_semantic_quota: 0,
                lexical_limit: 0,
                temporal_limit: 0,
            },
        )
        .unwrap();
    let calls = Arc::new(Mutex::new(Vec::new()));
    let classifier = DreamClassifier::new(RecordingEndpoint::new(calls));
    assert_eq!(classifier.classify_candidates(&set).unwrap().len(), 2);

    let none = SimulatedGeneralEndpoint::new(
        "dream-model",
        vec![json!({"relation": "none", "direction": "none", "evidence": []})],
    );
    let result = DreamClassifier::new(none)
        .classify_pair(&set.source, &set.candidates[0].context)
        .unwrap();
    assert_eq!(result.relation, DreamRelationKind::None);
    assert!(result.evidence.is_empty());
}
