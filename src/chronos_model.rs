#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum TemporalGranularity {
    Instant,
    Day,
    Week,
    Month,
    Quarter,
    Year,
    Range,
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum TemporalOrigin {
    Explicit,
    Relative,
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum TemporalDurationUnit {
    Second,
    Minute,
    Hour,
    Day,
    Week,
    Month,
    Quarter,
    Year,
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum TemporalFrequency {
    Daily,
    Weekly,
    Monthly,
    Quarterly,
    Yearly,
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum TemporalWeekday {
    Monday,
    Tuesday,
    Wednesday,
    Thursday,
    Friday,
    Saturday,
    Sunday,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TemporalAnchor {
    pub start_ns: i64,
    pub end_ns: i64,
    pub granularity: TemporalGranularity,
    pub origin: TemporalOrigin,
    pub evidence: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TemporalPattern {
    pub frequency: TemporalFrequency,
    pub interval: u16,
    pub weekday: Option<TemporalWeekday>,
    pub month_day: Option<u8>,
    pub month: Option<u8>,
    pub evidence: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TemporalDuration {
    pub amount: u16,
    pub unit: TemporalDurationUnit,
    pub evidence: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TemporalInterval {
    pub start_ns: Option<i64>,
    pub end_ns: Option<i64>,
    pub start_granularity: Option<TemporalGranularity>,
    pub end_granularity: Option<TemporalGranularity>,
    pub origin: TemporalOrigin,
    pub evidence: String,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct TemporalAnalysis {
    pub source_timestamp_ns: Option<i64>,
    pub anchors: Vec<TemporalAnchor>,
    pub durations: Vec<TemporalDuration>,
    pub intervals: Vec<TemporalInterval>,
    pub patterns: Vec<TemporalPattern>,
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum TemporalMatchKind {
    ExactInstant,
    ExactDay,
    ExactWeek,
    ExactMonth,
    ExactQuarter,
    ExactYear,
    RangeOverlap,
    Recurrence,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TemporalMatch {
    pub kind: TemporalMatchKind,
    pub source_anchor: Option<TemporalAnchor>,
    pub candidate_anchor: Option<TemporalAnchor>,
    pub source_pattern: Option<TemporalPattern>,
    pub candidate_pattern: Option<TemporalPattern>,
}
