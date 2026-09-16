use crate::{TemporalAnalysis, TemporalAnchor, TemporalInterval, TemporalPattern};

#[derive(Clone, Copy)]
pub(crate) struct TextSpan {
    pub(crate) start: usize,
    pub(crate) end: usize,
}

pub(crate) fn parse_temporal(text: &str, source_timestamp_ns: Option<i64>) -> TemporalAnalysis {
    let mut claimed = Vec::new();
    let mut anchors = Vec::new();
    let mut intervals = Vec::new();
    crate::chronos_boundary::extract_boundaries(
        text,
        source_timestamp_ns,
        &mut claimed,
        &mut anchors,
        &mut intervals,
    );
    crate::chronos_absolute::extract_absolute(text, &mut claimed, &mut anchors);
    crate::chronos_relative::extract_relative(
        text,
        source_timestamp_ns,
        &mut claimed,
        &mut anchors,
    );
    mirror_range_anchors(&anchors, &mut intervals);
    let mut patterns = crate::chronos_recurrence::extract_recurrence(text);
    normalize_anchors(&mut anchors);
    normalize_intervals(&mut intervals);
    normalize_patterns(&mut patterns);
    TemporalAnalysis {
        source_timestamp_ns,
        anchors,
        intervals,
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

pub(crate) fn merge_temporal_analysis(base: &mut TemporalAnalysis, mut extra: TemporalAnalysis) {
    extra.anchors.retain(|candidate| {
        !base.anchors.iter().any(|existing| {
            existing.start_ns == candidate.start_ns
                && existing.end_ns == candidate.end_ns
                && existing.granularity == candidate.granularity
                && existing.origin == candidate.origin
        })
    });
    extra.intervals.retain(|candidate| {
        !base.intervals.iter().any(|existing| {
            existing.start_ns == candidate.start_ns
                && existing.end_ns == candidate.end_ns
                && existing.start_granularity == candidate.start_granularity
                && existing.end_granularity == candidate.end_granularity
                && existing.origin == candidate.origin
        })
    });
    extra.patterns.retain(|candidate| {
        !base.patterns.iter().any(|existing| {
            existing.frequency == candidate.frequency
                && existing.interval == candidate.interval
                && existing.weekday == candidate.weekday
                && existing.month_day == candidate.month_day
                && existing.month == candidate.month
        })
    });
    base.anchors.append(&mut extra.anchors);
    base.intervals.append(&mut extra.intervals);
    base.patterns.append(&mut extra.patterns);
    normalize_anchors(&mut base.anchors);
    normalize_intervals(&mut base.intervals);
    normalize_patterns(&mut base.patterns);
}

fn mirror_range_anchors(anchors: &[TemporalAnchor], intervals: &mut Vec<TemporalInterval>) {
    for anchor in anchors
        .iter()
        .filter(|anchor| anchor.granularity == crate::TemporalGranularity::Range)
    {
        if intervals.iter().any(|interval| {
            interval.start_ns == Some(anchor.start_ns)
                && interval.end_ns == Some(anchor.end_ns)
                && interval.origin == anchor.origin
                && interval.evidence == anchor.evidence
        }) {
            continue;
        }
        intervals.push(TemporalInterval {
            start_ns: Some(anchor.start_ns),
            end_ns: Some(anchor.end_ns),
            start_granularity: Some(crate::TemporalGranularity::Range),
            end_granularity: Some(crate::TemporalGranularity::Range),
            origin: anchor.origin,
            evidence: anchor.evidence.clone(),
        });
    }
}

fn normalize_anchors(values: &mut Vec<TemporalAnchor>) {
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

fn normalize_intervals(values: &mut Vec<TemporalInterval>) {
    values.sort_by(|left, right| {
        (
            left.start_ns,
            left.end_ns,
            left.start_granularity,
            left.end_granularity,
            left.origin,
            &left.evidence,
        )
            .cmp(&(
                right.start_ns,
                right.end_ns,
                right.start_granularity,
                right.end_granularity,
                right.origin,
                &right.evidence,
            ))
    });
    values.dedup_by(|left, right| {
        left.start_ns == right.start_ns
            && left.end_ns == right.end_ns
            && left.start_granularity == right.start_granularity
            && left.end_granularity == right.end_granularity
            && left.origin == right.origin
    });
}

fn normalize_patterns(values: &mut Vec<TemporalPattern>) {
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
