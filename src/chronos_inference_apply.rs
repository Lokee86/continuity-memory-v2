use crate::{TemporalAnalysis, TemporalInference, TemporalInferenceError};

pub(crate) fn apply_inference(
    mut analysis: TemporalAnalysis,
    inference: &TemporalInference,
    reference_timestamp_ns: Option<i64>,
) -> Result<TemporalAnalysis, TemporalInferenceError> {
    if inference.contract_version != crate::CHRONOS_INFERENCE_CONTRACT_VERSION {
        return Err(TemporalInferenceError::InvalidOutput(
            "unsupported persisted inference contract version".into(),
        ));
    }
    for resolution in &inference.resolutions {
        let mut inferred = crate::chronos_inference_verify::verify_canonical_expression(
            &resolution.canonical_expression,
            resolution.kind,
            reference_timestamp_ns,
        )?;
        for anchor in &mut inferred.anchors {
            anchor.evidence = resolution.evidence.clone();
        }
        for duration in &mut inferred.durations {
            duration.evidence = resolution.evidence.clone();
        }
        for duration in &mut inferred.duration_ranges {
            duration.evidence = resolution.evidence.clone();
        }
        for duration in &mut inferred.approximate_durations {
            duration.evidence = resolution.evidence.clone();
        }
        for relation in &mut inferred.event_relations {
            relation.evidence = resolution.evidence.clone();
        }
        for interval in &mut inferred.intervals {
            interval.evidence = resolution.evidence.clone();
        }
        for pattern in &mut inferred.patterns {
            pattern.evidence = resolution.evidence.clone();
        }
        crate::chronos_parser::merge_temporal_analysis(&mut analysis, inferred);
    }
    Ok(analysis)
}
