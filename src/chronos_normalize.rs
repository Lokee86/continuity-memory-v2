use crate::{
    TemporalAnchor, TemporalDuration, TemporalInterval, TemporalPattern, TemporalTimeOfDay,
};

pub(crate) fn anchors(values: &mut Vec<TemporalAnchor>) {
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

pub(crate) fn times_of_day(values: &mut Vec<TemporalTimeOfDay>) {
    values.sort_by(|left, right| {
        (
            left.hour,
            left.minute,
            left.second,
            left.precision,
            left.origin,
            &left.evidence,
        )
            .cmp(&(
                right.hour,
                right.minute,
                right.second,
                right.precision,
                right.origin,
                &right.evidence,
            ))
    });
    values.dedup_by(|left, right| {
        left.hour == right.hour
            && left.minute == right.minute
            && left.second == right.second
            && left.precision == right.precision
            && left.origin == right.origin
    });
}

pub(crate) fn durations(values: &mut Vec<TemporalDuration>) {
    values.sort_by(|left, right| {
        (left.amount, left.unit, &left.evidence).cmp(&(right.amount, right.unit, &right.evidence))
    });
    values.dedup_by(|left, right| left.amount == right.amount && left.unit == right.unit);
}

pub(crate) fn intervals(values: &mut Vec<TemporalInterval>) {
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

pub(crate) fn patterns(values: &mut Vec<TemporalPattern>) {
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
