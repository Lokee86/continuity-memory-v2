use crate::{TemporalAnalysis, TemporalDetection, TemporalMatch};

/// Detect possible world-time material without requiring that Chronos can already parse it.
pub fn detect(text: &str) -> TemporalDetection {
    crate::chronos_detection::detect_and_normalize(text).detection
}

/// Deterministically analyze temporal language against an optional authoritative reference time.
///
/// Chronos does not choose the reference chronology. Consumers such as Dream and Insomnia resolve
/// their authoritative source/reference time and pass it through this shared boundary.
pub fn analyze(text: &str, reference_timestamp_ns: Option<i64>) -> TemporalAnalysis {
    let detected = crate::chronos_detection::detect_and_normalize(text);
    let mut analysis = crate::chronos_parser::parse_temporal(text, reference_timestamp_ns);
    let Some(normalized_text) = detected.normalized_text.as_deref() else {
        return analysis;
    };

    let mut corrected =
        crate::chronos_parser::parse_temporal(normalized_text, reference_timestamp_ns);
    for anchor in &mut corrected.anchors {
        anchor.evidence = detected.restore_evidence(text, &anchor.evidence);
    }
    for duration in &mut corrected.durations {
        duration.evidence = detected.restore_evidence(text, &duration.evidence);
    }
    for interval in &mut corrected.intervals {
        interval.evidence = detected.restore_evidence(text, &interval.evidence);
    }
    for pattern in &mut corrected.patterns {
        pattern.evidence = detected.restore_evidence(text, &pattern.evidence);
    }
    crate::chronos_parser::merge_temporal_analysis(&mut analysis, corrected);
    analysis
}

pub(crate) fn temporal_matches(
    source: &TemporalAnalysis,
    candidate: &TemporalAnalysis,
) -> (Vec<TemporalMatch>, f64) {
    crate::chronos_match::temporal_matches(source, candidate)
}
