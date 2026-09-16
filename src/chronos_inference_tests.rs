use crate::{GeneralEndpoint, GeneralEndpointError, TemporalInferencer, TemporalResolutionStatus};
use serde_json::{Value, json};
use std::sync::atomic::{AtomicUsize, Ordering};

struct CountingEndpoint {
    calls: AtomicUsize,
    response: Value,
}

impl CountingEndpoint {
    fn new(response: Value) -> Self {
        Self {
            calls: AtomicUsize::new(0),
            response,
        }
    }

    fn calls(&self) -> usize {
        self.calls.load(Ordering::SeqCst)
    }
}

impl GeneralEndpoint for CountingEndpoint {
    fn model(&self) -> &str {
        "chronos-test-model"
    }

    fn complete_json(
        &self,
        _system_prompt: &str,
        _user_payload: &str,
        schema_name: &str,
        _schema: &Value,
    ) -> Result<Value, GeneralEndpointError> {
        assert_eq!(schema_name, "chronos_temporal_inference");
        self.calls.fetch_add(1, Ordering::SeqCst);
        Ok(self.response.clone())
    }
}

#[test]
fn deterministic_temporal_material_never_calls_inference() {
    let assessment = crate::chronos::assess("Deadline 2027-03-04.", None);
    assert_eq!(
        assessment.resolution.status,
        TemporalResolutionStatus::FullyResolved
    );
    let endpoint = CountingEndpoint::new(json!({"resolutions": []}));
    let inferencer = TemporalInferencer::new(&endpoint);
    assert!(
        inferencer
            .infer("Deadline 2027-03-04.", None, &assessment)
            .unwrap()
            .is_none()
    );
    assert_eq!(endpoint.calls(), 0);
}

#[test]
fn unresolved_temporal_material_is_canonicalized_then_deterministically_verified() {
    let text = "Review biweekly.";
    let assessment = crate::chronos::assess(text, None);
    assert!(assessment.resolution.needs_inference());
    let endpoint = CountingEndpoint::new(json!({
        "resolutions": [{"id":"u000","canonical_expression":"every two weeks"}]
    }));
    let inference = TemporalInferencer::new(&endpoint)
        .infer(text, None, &assessment)
        .unwrap()
        .unwrap();
    assert_eq!(endpoint.calls(), 1);
    assert_eq!(inference.resolutions.len(), 1);
    assert_eq!(inference.resolutions[0].evidence, "biweekly");

    let analysis = crate::chronos::analyze_with_inference(text, None, &inference).unwrap();
    assert_eq!(analysis.patterns.len(), 1);
    assert_eq!(analysis.patterns[0].interval, 2);
    assert_eq!(analysis.patterns[0].evidence, "biweekly");
}

#[test]
fn inference_may_abstain_without_creating_durable_semantics() {
    let text = "Review biweekly.";
    let assessment = crate::chronos::assess(text, None);
    let endpoint = CountingEndpoint::new(json!({
        "resolutions": [{"id":"u000","canonical_expression":""}]
    }));
    let inference = TemporalInferencer::new(&endpoint)
        .infer(text, None, &assessment)
        .unwrap();
    assert!(inference.is_none());
    assert_eq!(endpoint.calls(), 1);
}

#[test]
fn inference_cannot_change_the_temporal_kind_of_unresolved_evidence() {
    let text = "Review biweekly.";
    let assessment = crate::chronos::assess(text, None);
    let endpoint = CountingEndpoint::new(json!({
        "resolutions": [{"id":"u000","canonical_expression":"March 2027"}]
    }));
    let error = TemporalInferencer::new(&endpoint)
        .infer(text, None, &assessment)
        .unwrap_err();
    assert!(error.to_string().contains("source temporal kind"));
    assert_eq!(endpoint.calls(), 1);
}

#[test]
fn inference_cannot_persist_an_expression_chronos_still_cannot_resolve() {
    let text = "Review biweekly.";
    let assessment = crate::chronos::assess(text, None);
    let endpoint = CountingEndpoint::new(json!({
        "resolutions": [{"id":"u000","canonical_expression":"biweekly"}]
    }));
    let error = TemporalInferencer::new(&endpoint)
        .infer(text, None, &assessment)
        .unwrap_err();
    assert!(
        error
            .to_string()
            .contains("not deterministically resolvable")
    );
    assert_eq!(endpoint.calls(), 1);
}
