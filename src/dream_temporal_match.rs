use crate::{
    DreamTemporalAnalysis, DreamTemporalAnchor, DreamTemporalGranularity, DreamTemporalMatch,
    DreamTemporalMatchKind, DreamTemporalPattern,
};

const MAX_TEMPORAL_MATCHES: usize = 6;

pub(crate) fn temporal_matches(
    source: &DreamTemporalAnalysis,
    candidate: &DreamTemporalAnalysis,
) -> (Vec<DreamTemporalMatch>, f64) {
    let mut matches = Vec::new();
    let mut best = 0.0_f64;
    for left in &source.anchors {
        for right in &candidate.anchors {
            if !overlap(left, right) {
                continue;
            }
            let (kind, score) = classify_anchor_match(left, right);
            best = best.max(score);
            matches.push(DreamTemporalMatch {
                kind,
                source_anchor: Some(left.clone()),
                candidate_anchor: Some(right.clone()),
                source_pattern: None,
                candidate_pattern: None,
            });
            if matches.len() == MAX_TEMPORAL_MATCHES {
                return (matches, best);
            }
        }
    }
    for left in &source.patterns {
        for right in &candidate.patterns {
            if !same_pattern(left, right) {
                continue;
            }
            best = best.max(0.7);
            matches.push(DreamTemporalMatch {
                kind: DreamTemporalMatchKind::Recurrence,
                source_anchor: None,
                candidate_anchor: None,
                source_pattern: Some(left.clone()),
                candidate_pattern: Some(right.clone()),
            });
            if matches.len() == MAX_TEMPORAL_MATCHES {
                return (matches, best);
            }
        }
    }
    (matches, best)
}

fn overlap(left: &DreamTemporalAnchor, right: &DreamTemporalAnchor) -> bool {
    left.start_ns < right.end_ns && right.start_ns < left.end_ns
}

fn classify_anchor_match(
    left: &DreamTemporalAnchor,
    right: &DreamTemporalAnchor,
) -> (DreamTemporalMatchKind, f64) {
    if left.start_ns == right.start_ns
        && left.end_ns == right.end_ns
        && left.granularity == right.granularity
    {
        return match left.granularity {
            DreamTemporalGranularity::Instant => (DreamTemporalMatchKind::ExactInstant, 1.0),
            DreamTemporalGranularity::Day => (DreamTemporalMatchKind::ExactDay, 0.98),
            DreamTemporalGranularity::Week => (DreamTemporalMatchKind::ExactWeek, 0.92),
            DreamTemporalGranularity::Month => (DreamTemporalMatchKind::ExactMonth, 0.88),
            DreamTemporalGranularity::Quarter => (DreamTemporalMatchKind::ExactQuarter, 0.84),
            DreamTemporalGranularity::Year => (DreamTemporalMatchKind::ExactYear, 0.8),
            DreamTemporalGranularity::Range => (DreamTemporalMatchKind::RangeOverlap, 0.96),
        };
    }
    (DreamTemporalMatchKind::RangeOverlap, 0.75)
}

fn same_pattern(left: &DreamTemporalPattern, right: &DreamTemporalPattern) -> bool {
    left.frequency == right.frequency
        && left.interval == right.interval
        && left.weekday == right.weekday
        && left.month_day == right.month_day
        && left.month == right.month
}
