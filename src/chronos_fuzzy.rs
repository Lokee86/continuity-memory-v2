use crate::TemporalIndicationKind;
use crate::chronos_edit_distance::osa_distance;
use crate::chronos_vocabulary::{calendar_context, unit_context, weekday_context};

const RELATIVE: &[&str] = &["tomorrow", "yesterday"];
const MONTHS: &[&str] = &[
    "january",
    "february",
    "march",
    "april",
    "june",
    "july",
    "august",
    "september",
    "october",
    "november",
    "december",
];
const WEEKDAYS: &[&str] = &[
    "monday",
    "tuesday",
    "wednesday",
    "thursday",
    "friday",
    "saturday",
    "sunday",
];
const UNITS: &[&str] = &[
    "month", "months", "week", "weeks", "year", "years", "hours", "minutes", "seconds", "quarter",
];
const RECURRENCE: &[&str] = &["quarterly", "monthly", "weekly", "yearly", "annually"];

pub(crate) fn fuzzy_word(
    word: &str,
    previous: Option<&str>,
    next: Option<&str>,
    capitalized: bool,
) -> Option<(&'static str, TemporalIndicationKind)> {
    if word.len() < 5 || !word.bytes().all(|byte| byte.is_ascii_alphabetic()) {
        return None;
    }
    let lower = word.to_ascii_lowercase();
    let mut matches = Vec::new();
    collect_matches(
        &lower,
        RELATIVE,
        TemporalIndicationKind::Relative,
        true,
        &mut matches,
    );
    collect_matches(
        &lower,
        MONTHS,
        TemporalIndicationKind::Calendar,
        calendar_context(previous, next, capitalized),
        &mut matches,
    );
    collect_matches(
        &lower,
        WEEKDAYS,
        TemporalIndicationKind::Calendar,
        weekday_context(previous, capitalized),
        &mut matches,
    );
    collect_matches(
        &lower,
        UNITS,
        TemporalIndicationKind::Duration,
        unit_context(previous, next),
        &mut matches,
    );
    collect_matches(
        &lower,
        RECURRENCE,
        TemporalIndicationKind::Recurrence,
        true,
        &mut matches,
    );
    if matches.len() == 1 {
        Some(matches[0])
    } else {
        None
    }
}

fn collect_matches(
    word: &str,
    candidates: &'static [&'static str],
    kind: TemporalIndicationKind,
    context_allowed: bool,
    matches: &mut Vec<(&'static str, TemporalIndicationKind)>,
) {
    if !context_allowed {
        return;
    }
    for &candidate in candidates {
        if word == candidate || word.len().abs_diff(candidate.len()) > 1 {
            continue;
        }
        if osa_distance(word, candidate) == 1 {
            matches.push((candidate, kind));
        }
    }
}
