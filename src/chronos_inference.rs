use crate::{
    GeneralEndpoint, TemporalAssessment, TemporalIndicationKind, TemporalInference,
    TemporalInferenceError, TemporalInferenceResolution,
};
use serde_json::{Value, json};
use std::collections::HashMap;

pub const CHRONOS_INFERENCE_CONTRACT_VERSION: &str = "chronos-temporal-inference-v1";
pub const CHRONOS_INFERENCE_SYSTEM_PROMPT: &str = r#"You resolve only temporal language that deterministic Chronos left unresolved.
Do not reinterpret or repeat temporal material that is already resolved.
For each supplied unresolved item, return exactly one result with the same id.
Set canonical_expression to a concise standalone temporal expression only when the source context supports one safe interpretation. The expression must preserve the source meaning and must not invent precision, geography, cadence, dates, or timezone. If the meaning is genuinely ambiguous or unsupported, return an empty canonical_expression.
Examples of acceptable canonical expressions include "every two weeks", "northern spring 2027", "March 4 2027", and "after 2027-03-04". Chronos will deterministically verify every non-empty expression before it can become durable."#;

pub struct TemporalInferencer<E> {
    endpoint: E,
}

impl<E: GeneralEndpoint> TemporalInferencer<E> {
    pub fn new(endpoint: E) -> Self {
        Self { endpoint }
    }

    pub fn model(&self) -> &str {
        self.endpoint.model()
    }

    pub fn infer(
        &self,
        text: &str,
        reference_timestamp_ns: Option<i64>,
        assessment: &TemporalAssessment,
    ) -> Result<Option<TemporalInference>, TemporalInferenceError> {
        if !assessment.resolution.needs_inference() {
            return Ok(None);
        }
        let unresolved = &assessment.resolution.unresolved_indications;
        if unresolved.len() > 32 {
            return Err(TemporalInferenceError::InvalidOutput(
                "more than 32 unresolved indications".into(),
            ));
        }
        let items = unresolved
            .iter()
            .enumerate()
            .map(|(index, indication)| {
                json!({
                    "id": format!("u{index:03}"),
                    "kind": indication_kind(indication.kind),
                    "start_byte": indication.start_byte,
                    "end_byte": indication.end_byte,
                    "evidence": indication.evidence,
                })
            })
            .collect::<Vec<_>>();
        let payload = json!({
            "semantic_text": text,
            "reference_timestamp_ns": reference_timestamp_ns,
            "deterministic_resolved_evidence": resolved_evidence(assessment),
            "unresolved": items,
        });
        let payload = serde_json::to_string(&payload)
            .map_err(|error| TemporalInferenceError::InvalidOutput(error.to_string()))?;
        let value = self.endpoint.complete_json(
            CHRONOS_INFERENCE_SYSTEM_PROMPT,
            &payload,
            "chronos_temporal_inference",
            &schema(),
        )?;
        let returned = parse_results(&value, unresolved.len())?;
        let mut resolutions = Vec::new();
        for (index, indication) in unresolved.iter().enumerate() {
            let canonical_expression = returned
                .get(&format!("u{index:03}"))
                .expect("validated result coverage")
                .trim();
            if canonical_expression.is_empty() {
                continue;
            }
            crate::chronos_inference_verify::verify_canonical_expression(
                canonical_expression,
                indication.kind,
                reference_timestamp_ns,
            )
            .map_err(|_| {
                TemporalInferenceError::InvalidOutput(format!(
                    "canonical expression for u{index:03} is not deterministically resolvable as the source temporal kind"
                ))
            })?;
            resolutions.push(TemporalInferenceResolution {
                start_byte: indication.start_byte,
                end_byte: indication.end_byte,
                kind: indication.kind,
                evidence: indication.evidence.clone(),
                canonical_expression: canonical_expression.to_owned(),
            });
        }
        Ok((!resolutions.is_empty()).then(|| TemporalInference {
            model: self.endpoint.model().to_owned(),
            contract_version: CHRONOS_INFERENCE_CONTRACT_VERSION.into(),
            resolutions,
        }))
    }
}

fn parse_results(
    value: &Value,
    expected: usize,
) -> Result<HashMap<String, String>, TemporalInferenceError> {
    let entries = value
        .get("resolutions")
        .and_then(Value::as_array)
        .ok_or_else(|| TemporalInferenceError::InvalidOutput("missing resolutions array".into()))?;
    if entries.len() != expected {
        return Err(TemporalInferenceError::InvalidOutput(
            "result count does not match unresolved indication count".into(),
        ));
    }
    let mut results = HashMap::new();
    for entry in entries {
        let id = entry.get("id").and_then(Value::as_str).ok_or_else(|| {
            TemporalInferenceError::InvalidOutput("resolution is missing id".into())
        })?;
        let canonical = entry
            .get("canonical_expression")
            .and_then(Value::as_str)
            .ok_or_else(|| {
                TemporalInferenceError::InvalidOutput(
                    "resolution is missing canonical_expression".into(),
                )
            })?;
        let Some(index) = id
            .strip_prefix('u')
            .and_then(|value| value.parse::<usize>().ok())
        else {
            return Err(TemporalInferenceError::InvalidOutput(
                "resolution id is invalid".into(),
            ));
        };
        if index >= expected
            || results
                .insert(id.to_owned(), canonical.to_owned())
                .is_some()
        {
            return Err(TemporalInferenceError::InvalidOutput(
                "resolution id is unknown or duplicated".into(),
            ));
        }
    }
    for index in 0..expected {
        if !results.contains_key(&format!("u{index:03}")) {
            return Err(TemporalInferenceError::InvalidOutput(
                "resolution id coverage is incomplete".into(),
            ));
        }
    }
    Ok(results)
}

fn resolved_evidence(assessment: &TemporalAssessment) -> Vec<String> {
    assessment
        .analysis
        .anchors
        .iter()
        .map(|value| value.evidence.clone())
        .chain(
            assessment
                .analysis
                .times_of_day
                .iter()
                .map(|value| value.evidence.clone()),
        )
        .chain(
            assessment
                .analysis
                .durations
                .iter()
                .map(|value| value.evidence.clone()),
        )
        .chain(
            assessment
                .analysis
                .duration_ranges
                .iter()
                .map(|value| value.evidence.clone()),
        )
        .chain(
            assessment
                .analysis
                .approximate_durations
                .iter()
                .map(|value| value.evidence.clone()),
        )
        .chain(
            assessment
                .analysis
                .event_relations
                .iter()
                .map(|value| value.evidence.clone()),
        )
        .chain(
            assessment
                .analysis
                .intervals
                .iter()
                .map(|value| value.evidence.clone()),
        )
        .chain(
            assessment
                .analysis
                .patterns
                .iter()
                .map(|value| value.evidence.clone()),
        )
        .collect()
}

fn indication_kind(kind: TemporalIndicationKind) -> &'static str {
    match kind {
        TemporalIndicationKind::Explicit => "explicit",
        TemporalIndicationKind::Calendar => "calendar",
        TemporalIndicationKind::Relative => "relative",
        TemporalIndicationKind::Boundary => "boundary",
        TemporalIndicationKind::Recurrence => "recurrence",
        TemporalIndicationKind::Duration => "duration",
    }
}

fn schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "resolutions": {
                "type": "array",
                "items": {
                    "type": "object",
                    "properties": {
                        "id": {"type": "string"},
                        "canonical_expression": {"type": "string"}
                    },
                    "required": ["id", "canonical_expression"],
                    "additionalProperties": false
                }
            }
        },
        "required": ["resolutions"],
        "additionalProperties": false
    })
}
