use crate::{TemporalAnalysis, TemporalMatch};

/// Deterministically analyze temporal language against an optional authoritative reference time.
///
/// Chronos does not choose the reference chronology. Consumers such as Dream and Insomnia resolve
/// their authoritative source/reference time and pass it through this shared boundary.
pub fn analyze(text: &str, reference_timestamp_ns: Option<i64>) -> TemporalAnalysis {
    crate::chronos_parser::parse_temporal(text, reference_timestamp_ns)
}

pub(crate) fn temporal_matches(
    source: &TemporalAnalysis,
    candidate: &TemporalAnalysis,
) -> (Vec<TemporalMatch>, f64) {
    crate::chronos_match::temporal_matches(source, candidate)
}
