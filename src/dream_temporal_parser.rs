use crate::{DreamTemporalAnalysis, DreamTemporalAnchor, DreamTemporalPattern};

#[derive(Clone, Copy)]
pub(crate) struct TextSpan {
    pub(crate) start: usize,
    pub(crate) end: usize,
}

pub(crate) fn parse_temporal(
    text: &str,
    source_timestamp_ns: Option<i64>,
) -> DreamTemporalAnalysis {
    let mut claimed = Vec::new();
    let mut anchors = Vec::new();
    crate::dream_temporal_absolute::extract_absolute(text, &mut claimed, &mut anchors);
    crate::dream_temporal_relative::extract_relative(
        text,
        source_timestamp_ns,
        &mut claimed,
        &mut anchors,
    );
    let mut patterns = crate::dream_temporal_recurrence::extract_recurrence(text);
    normalize_anchors(&mut anchors);
    normalize_patterns(&mut patterns);
    DreamTemporalAnalysis {
        source_timestamp_ns,
        anchors,
        patterns,
    }
}

pub(crate) fn claimed(span: TextSpan, claimed: &[TextSpan]) -> bool {
    claimed
        .iter()
        .any(|other| span.start < other.end && other.start < span.end)
}

pub(crate) fn push_claimed(claimed_spans: &mut Vec<TextSpan>, span: TextSpan) {
    claimed_spans.push(span);
}

fn normalize_anchors(values: &mut Vec<DreamTemporalAnchor>) {
    values.sort_by(|left, right| {
        (
            left.start_ns,
            left.end_ns,
            left.granularity,
            left.origin,
            &left.evidence,
        )
            .cmp(&(
                right.start_ns,
                right.end_ns,
                right.granularity,
                right.origin,
                &right.evidence,
            ))
    });
    values.dedup_by(|left, right| {
        left.start_ns == right.start_ns
            && left.end_ns == right.end_ns
            && left.granularity == right.granularity
            && left.origin == right.origin
    });
}

fn normalize_patterns(values: &mut Vec<DreamTemporalPattern>) {
    values.sort_by(|left, right| {
        (
            left.frequency,
            left.interval,
            left.weekday,
            left.month_day,
            left.month,
            &left.evidence,
        )
            .cmp(&(
                right.frequency,
                right.interval,
                right.weekday,
                right.month_day,
                right.month,
                &right.evidence,
            ))
    });
    values.dedup_by(|left, right| {
        left.frequency == right.frequency
            && left.interval == right.interval
            && left.weekday == right.weekday
            && left.month_day == right.month_day
            && left.month == right.month
    });
}
