use crate::dream_candidate_test_support::{install_vectors, memory, memory_extracted, test_path};
use crate::{
    Cva, DreamCandidateConfig, DreamProcessError, DreamProcessor, DreamVerificationPolicy,
    GeneralEndpoint, GeneralEndpointError, SimulatedGeneralEndpoint,
};
use serde_json::{Value, json};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;
use std::time::Duration;

#[test]
fn complete_pass_with_no_candidates_advances_source_to_knowledge() {
    let mut cva = Cva::create(test_path("processor-empty.cva")).unwrap();
    let source = memory_extracted(&mut cva, "source", "Source", "Only memory.", 100);
    let profile = install_vectors(&mut cva, &[source], &[&[1.0, 0.0]]);
    let processor = DreamProcessor::new(
        SimulatedGeneralEndpoint::new("classifier", vec![]),
        SimulatedGeneralEndpoint::new("verifier", vec![]),
    );

    let result = processor
        .process_memory(
            &mut cva,
            profile,
            source,
            DreamCandidateConfig::default(),
            DreamVerificationPolicy::default(),
        )
        .unwrap();

    assert_eq!(result.candidate_count, 0);
    assert!(result.pairs.is_empty());
    assert!(result.lifecycle.promoted_to_knowledge);
    assert_eq!(result.source.lifecycle_state, "knowledge");
}

#[test]
fn complete_duplicate_pass_publishes_chain_then_archives_source() {
    let mut cva = Cva::create(test_path("processor-duplicate.cva")).unwrap();
    let _representative = memory(&mut cva, "rep", "Same", "Same memory.", 100, false);
    let source = memory_extracted(&mut cva, "source", "Same", "Same memory.", 110);
    let profile = install_vectors(&mut cva, &[source], &[&[1.0, 0.0]]);
    let classifier = SimulatedGeneralEndpoint::new(
        "classifier",
        vec![json!({
            "relation": "duplicate_of",
            "direction": "undirected",
            "evidence": [
                {"side": "a", "quote": "Same memory."},
                {"side": "b", "quote": "Same memory."}
            ]
        })],
    );
    let verifier = SimulatedGeneralEndpoint::new(
        "verifier",
        vec![json!({
            "relation_supported": "yes",
            "direction_supported": "yes",
            "evidence_supported": "yes"
        })],
    );
    let processor = DreamProcessor::new(classifier, verifier);

    let result = processor
        .process_memory(
            &mut cva,
            profile,
            source,
            DreamCandidateConfig::default(),
            DreamVerificationPolicy::default(),
        )
        .unwrap();

    assert_eq!(result.candidate_count, 1);
    assert_eq!(result.pairs.len(), 1);
    assert!(result.source.archived);
    assert_eq!(result.source.lifecycle_state, "archived");
    assert_eq!(cva.graph_relations().len(), 1);
}

#[test]
fn complete_supersession_pass_archives_replaced_memory_and_advances_source() {
    let mut cva = Cva::create(test_path("processor-supersedes.cva")).unwrap();
    let old = memory(&mut cva, "old", "Old", "Rule old.", 100, false);
    let source = memory_extracted(&mut cva, "source", "New", "Rule new.", 110);
    let profile = install_vectors(&mut cva, &[source, old], &[&[1.0, 0.0], &[0.95, 0.05]]);
    let direction = if source.0 < old.0 { "a_to_b" } else { "b_to_a" };
    let classifier = SimulatedGeneralEndpoint::new(
        "classifier",
        vec![json!({
            "relation": "supersedes",
            "direction": direction,
            "evidence": [
                {"side": "a", "quote": "Rule"},
                {"side": "b", "quote": "Rule"}
            ]
        })],
    );
    let verifier = SimulatedGeneralEndpoint::new(
        "verifier",
        vec![json!({
            "relation_supported": "yes",
            "direction_supported": "yes",
            "evidence_supported": "yes"
        })],
    );
    let processor = DreamProcessor::new(classifier, verifier);

    let result = processor
        .process_memory(
            &mut cva,
            profile,
            source,
            DreamCandidateConfig::default(),
            DreamVerificationPolicy::default(),
        )
        .unwrap();

    let old = cva.memory(old).unwrap();
    assert!(old.archived);
    assert_eq!(old.lifecycle_state, "archived");
    assert_eq!(old.superseded_by, Some(source));
    assert_eq!(result.source.lifecycle_state, "knowledge");
    assert!(!result.source.archived);
}

#[test]
fn temporal_only_candidate_flows_through_complete_dream_processing() {
    let mut cva = Cva::create(test_path("processor-temporal.cva")).unwrap();
    let candidate = memory(
        &mut cva,
        "candidate",
        "Field event",
        "Inspection scheduled for 2026-08-25.",
        1,
        false,
    );
    let source = memory_extracted(
        &mut cva,
        "source",
        "Upcoming inspection",
        "Inspection is tomorrow.",
        1_787_572_800_000_000_000,
    );
    let profile = install_vectors(&mut cva, &[source], &[&[1.0, 0.0]]);
    let classifier = SimulatedGeneralEndpoint::new(
        "classifier",
        vec![json!({
            "relation": "topical",
            "direction": "undirected",
            "evidence": [
                {"side": "a", "quote": "Inspection"},
                {"side": "b", "quote": "Inspection"}
            ]
        })],
    );
    let processor = DreamProcessor::new(
        classifier,
        SimulatedGeneralEndpoint::new("verifier", vec![]),
    );

    let result = processor
        .process_memory(
            &mut cva,
            profile,
            source,
            DreamCandidateConfig {
                limit: 1,
                semantic_limit: 0,
                prior_semantic_quota: 0,
                lexical_limit: 0,
                temporal_limit: 1,
            },
            DreamVerificationPolicy::default(),
        )
        .unwrap();

    assert_eq!(result.candidate_count, 1);
    assert_eq!(result.pairs.len(), 1);
    assert_eq!(result.source.lifecycle_state, "knowledge");
    assert_eq!(cva.graph_relations().len(), 2);
    assert_eq!(cva.memory(candidate).unwrap().lifecycle_state, "knowledge");
}

#[derive(Clone)]
struct TrackingClassifier {
    active: Arc<AtomicUsize>,
    peak: Arc<AtomicUsize>,
}

impl GeneralEndpoint for TrackingClassifier {
    fn model(&self) -> &str {
        "tracking-classifier"
    }

    fn complete_json(
        &self,
        _system_prompt: &str,
        _user_payload: &str,
        _schema_name: &str,
        _schema: &Value,
    ) -> Result<Value, GeneralEndpointError> {
        let active = self.active.fetch_add(1, Ordering::SeqCst) + 1;
        self.peak.fetch_max(active, Ordering::SeqCst);
        thread::sleep(Duration::from_millis(40));
        self.active.fetch_sub(1, Ordering::SeqCst);
        Ok(json!({"relation": "none", "direction": "none", "evidence": []}))
    }
}

#[test]
fn pair_inference_can_run_concurrently_while_publication_remains_ordered() {
    let mut cva = Cva::create(test_path("processor-parallel.cva")).unwrap();
    let source = memory_extracted(&mut cva, "source", "Source", "Shared topic source.", 200);
    let a = memory(&mut cva, "a", "A", "Shared topic A.", 100, false);
    let b = memory(&mut cva, "b", "B", "Shared topic B.", 110, false);
    let c = memory(&mut cva, "c", "C", "Shared topic C.", 120, false);
    let profile = install_vectors(
        &mut cva,
        &[source, a, b, c],
        &[&[1.0, 0.0], &[0.99, 0.01], &[0.98, 0.02], &[0.97, 0.03]],
    );
    let active = Arc::new(AtomicUsize::new(0));
    let peak = Arc::new(AtomicUsize::new(0));
    let processor = DreamProcessor::new(
        TrackingClassifier {
            active: active.clone(),
            peak: peak.clone(),
        },
        SimulatedGeneralEndpoint::new("verifier", vec![]),
    );

    let result = processor
        .process_memory_with_pair_concurrency(
            &mut cva,
            profile,
            source,
            DreamCandidateConfig {
                limit: 3,
                semantic_limit: 3,
                prior_semantic_quota: 0,
                lexical_limit: 0,
                temporal_limit: 0,
            },
            DreamVerificationPolicy::default(),
            3,
        )
        .unwrap();

    assert_eq!(result.candidate_count, 3);
    assert_eq!(result.pairs.len(), 3);
    assert!(peak.load(Ordering::SeqCst) >= 2);
    assert_eq!(result.source.lifecycle_state, "knowledge");
}

#[test]
fn inference_failure_does_not_advance_source_lifecycle() {
    let mut cva = Cva::create(test_path("processor-failure.cva")).unwrap();
    let candidate = memory(
        &mut cva,
        "candidate",
        "Candidate",
        "Candidate memory.",
        100,
        false,
    );
    let source = memory_extracted(&mut cva, "source", "Source", "Source memory.", 110);
    let profile = install_vectors(&mut cva, &[source, candidate], &[&[1.0, 0.0], &[0.9, 0.1]]);
    let processor = DreamProcessor::new(
        SimulatedGeneralEndpoint::new("classifier", vec![]),
        SimulatedGeneralEndpoint::new("verifier", vec![]),
    );

    assert!(matches!(
        processor.process_memory(
            &mut cva,
            profile,
            source,
            DreamCandidateConfig::default(),
            DreamVerificationPolicy::default(),
        ),
        Err(DreamProcessError::Classification(_))
    ));
    let source = cva.memory(source).unwrap();
    assert_eq!(source.lifecycle_state, "extracted");
    assert!(!source.archived);
}
