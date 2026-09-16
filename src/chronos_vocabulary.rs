use crate::TemporalIndicationKind;

pub(crate) fn exact_kind(
    word: &str,
    previous: Option<&str>,
    next: Option<&str>,
    capitalized: bool,
) -> Option<TemporalIndicationKind> {
    let word = word.to_ascii_lowercase();
    let value = match word.as_str() {
        "today" | "tomorrow" | "yesterday" | "ago" => TemporalIndicationKind::Relative,
        "since" | "until" | "before" | "after" | "starting" | "ending" | "through" => {
            TemporalIndicationKind::Boundary
        }
        "every" | "daily" | "weekly" | "monthly" | "quarterly" | "yearly" | "annually"
        | "biweekly" | "bimonthly" | "semiweekly" | "semimonthly" => {
            TemporalIndicationKind::Recurrence
        }
        value if is_calendar_word(value) && calendar_context(previous, next, capitalized) => {
            TemporalIndicationKind::Calendar
        }
        value if is_unit(value) && unit_context(previous, next) => TemporalIndicationKind::Duration,
        "this" | "next" | "last" if next.is_some_and(is_temporal_follow) => {
            TemporalIndicationKind::Relative
        }
        _ => return None,
    };
    Some(value)
}

pub(crate) fn unit_context(previous: Option<&str>, next: Option<&str>) -> bool {
    previous.is_some_and(is_number_or_relative_cue) || next.is_some_and(is_relative_suffix)
}

pub(crate) fn calendar_context(
    previous: Option<&str>,
    next: Option<&str>,
    capitalized: bool,
) -> bool {
    capitalized || previous.is_some_and(is_calendar_cue) || next.is_some_and(is_ascii_number)
}

pub(crate) fn weekday_context(previous: Option<&str>, capitalized: bool) -> bool {
    capitalized
        || previous
            .is_some_and(|value| matches!(value, "next" | "last" | "this" | "every" | "on" | "by"))
}

fn is_number_or_relative_cue(value: &str) -> bool {
    is_ascii_number(value)
        || is_word_number(value)
        || matches!(
            value,
            "in" | "next" | "last" | "this" | "every" | "for" | "past"
        )
}

fn is_relative_suffix(value: &str) -> bool {
    matches!(value, "ago" | "later" | "earlier" | "from" | "until")
}

fn is_calendar_cue(value: &str) -> bool {
    matches!(
        value,
        "in" | "during" | "by" | "until" | "since" | "from" | "next" | "last"
    )
}

fn is_temporal_follow(value: &str) -> bool {
    is_unit(value) || is_calendar_word(value)
}

fn is_unit(value: &str) -> bool {
    matches!(
        value,
        "day"
            | "days"
            | "week"
            | "weeks"
            | "month"
            | "months"
            | "quarter"
            | "quarters"
            | "year"
            | "years"
            | "hour"
            | "hours"
            | "minute"
            | "minutes"
            | "second"
            | "seconds"
    )
}

fn is_calendar_word(value: &str) -> bool {
    matches!(
        value,
        "january"
            | "february"
            | "march"
            | "april"
            | "may"
            | "june"
            | "july"
            | "august"
            | "september"
            | "october"
            | "november"
            | "december"
            | "monday"
            | "tuesday"
            | "wednesday"
            | "thursday"
            | "friday"
            | "saturday"
            | "sunday"
            | "spring"
            | "summer"
            | "autumn"
            | "fall"
            | "winter"
    )
}

fn is_ascii_number(value: &str) -> bool {
    !value.is_empty() && value.bytes().all(|byte| byte.is_ascii_digit())
}

fn is_word_number(value: &str) -> bool {
    matches!(
        value,
        "one"
            | "two"
            | "three"
            | "four"
            | "five"
            | "six"
            | "seven"
            | "eight"
            | "nine"
            | "ten"
            | "eleven"
            | "twelve"
            | "thirteen"
            | "fourteen"
            | "fifteen"
            | "sixteen"
            | "seventeen"
            | "eighteen"
            | "nineteen"
            | "twenty"
            | "thirty"
            | "forty"
            | "fifty"
            | "sixty"
            | "seventy"
            | "eighty"
            | "ninety"
    )
}
