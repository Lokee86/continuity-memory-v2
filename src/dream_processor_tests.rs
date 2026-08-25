use crate::dream_candidate_test_support::{install_vectors, memory, memory_extracted, test_path};
use crate::{
    Cva, DreamCandidateConfig, DreamProcessError, DreamProcessor, DreamVerificationPolicy,
    SimulatedGeneralEndpoint,
};
use serde_json::json;

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
