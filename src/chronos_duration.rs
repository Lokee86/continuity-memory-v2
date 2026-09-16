use crate::chronos_number::{WORD_NUMBER_RE, parse_number};
use crate::{TemporalDuration, TemporalDurationUnit};
use regex::Regex;
use std::sync::LazyLock;

static DURATION_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(&format!(
        r"(?i)\b(?:for|lasting|lasted|duration(?:\s+of)?|takes?|took)\s+({number}|\d{{1,4}})\s+(second|minute|hour|day|week|month|quarter|year)s?\b",
        number = WORD_NUMBER_RE
    ))
    .expect("valid Chronos duration regex")
});

pub(crate) fn extract_durations(text: &str) -> Vec<TemporalDuration> {
    DURATION_RE
        .captures_iter(text)
        .filter_map(|captures| {
            let found = captures.get(0)?;
            let amount = parse_number(captures.get(1)?.as_str())?;
            let amount = u16::try_from(amount).ok().filter(|value| *value > 0)?;
            let unit = duration_unit(captures.get(2)?.as_str())?;
            Some(TemporalDuration {
                amount,
                unit,
                evidence: found.as_str().to_owned(),
            })
        })
        .collect()
}

fn duration_unit(value: &str) -> Option<TemporalDurationUnit> {
    Some(match value.to_ascii_lowercase().as_str() {
        "second" => TemporalDurationUnit::Second,
        "minute" => TemporalDurationUnit::Minute,
        "hour" => TemporalDurationUnit::Hour,
        "day" => TemporalDurationUnit::Day,
        "week" => TemporalDurationUnit::Week,
        "month" => TemporalDurationUnit::Month,
        "quarter" => TemporalDurationUnit::Quarter,
        "year" => TemporalDurationUnit::Year,
        _ => return None,
    })
}
