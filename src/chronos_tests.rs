use crate::chronos::{analyze, temporal_matches};
use crate::{TemporalFrequency, TemporalGranularity, TemporalMatchKind, TemporalOrigin};
use time::{Date, Month};

#[test]
fn shared_chronos_api_preserves_deterministic_absolute_relative_and_recurrence_behavior() {
    let reference = timestamp_ns(2026, Month::August, 24, 12);
    let analysis = analyze(
        "Deadline 2026-09-15. Review tomorrow. Crew reports every Monday.",
        Some(reference),
    );

    assert_eq!(analysis.source_timestamp_ns, Some(reference));
    assert!(analysis.anchors.iter().any(|anchor| {
        anchor.granularity == TemporalGranularity::Day
            && anchor.origin == TemporalOrigin::Explicit
            && anchor.start_ns == day_start_ns(2026, Month::September, 15)
    }));
    assert!(analysis.anchors.iter().any(|anchor| {
        anchor.granularity == TemporalGranularity::Day
            && anchor.origin == TemporalOrigin::Relative
            && anchor.start_ns == day_start_ns(2026, Month::August, 25)
    }));
    assert!(analysis.patterns.iter().any(|pattern| {
        pattern.frequency == TemporalFrequency::Weekly && pattern.interval == 1
    }));
}

#[test]
fn unicode_dash_date_ranges_remain_supported() {
    let analysis = analyze("Window 2026-10-01 – 2026-10-03.", None);
    assert!(analysis.anchors.iter().any(|anchor| {
        anchor.granularity == TemporalGranularity::Range
            && anchor.start_ns == day_start_ns(2026, Month::October, 1)
            && anchor.end_ns == day_start_ns(2026, Month::October, 4)
    }));
}

#[test]
fn shared_chronos_matcher_preserves_exact_day_scoring() {
    let reference = timestamp_ns(2026, Month::August, 24, 12);
    let relative = analyze("Inspection is tomorrow.", Some(reference));
    let explicit = analyze("Inspection is 2026-08-25.", None);

    let (matches, score) = temporal_matches(&relative, &explicit);
    assert!(score > 0.9);
    assert!(
        matches
            .iter()
            .any(|matched| matched.kind == TemporalMatchKind::ExactDay)
    );
}

fn timestamp_ns(year: i32, month: Month, day: u8, hour: u8) -> i64 {
    let date = Date::from_calendar_date(year, month, day).unwrap();
    let value = date.with_hms(hour, 0, 0).unwrap().assume_utc();
    i64::try_from(value.unix_timestamp_nanos()).unwrap()
}

fn day_start_ns(year: i32, month: Month, day: u8) -> i64 {
    let date = Date::from_calendar_date(year, month, day).unwrap();
    i64::try_from(date.midnight().assume_utc().unix_timestamp_nanos()).unwrap()
}
