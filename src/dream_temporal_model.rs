#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum DreamTemporalGranularity {
    Instant,
    Day,
    Week,
    Month,
    Quarter,
    Year,
    Range,
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum DreamTemporalOrigin {
    Explicit,
    Relative,
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum DreamTemporalFrequency {
    Daily,
    Weekly,
    Monthly,
    Quarterly,
    Yearly,
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum DreamTemporalWeekday {
    Monday,
    Tuesday,
    Wednesday,
    Thursday,
    Friday,
    Saturday,
    Sunday,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DreamTemporalAnchor {
    pub start_ns: i64,
    pub end_ns: i64,
    pub granularity: DreamTemporalGranularity,
    pub origin: DreamTemporalOrigin,
    pub evidence: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DreamTemporalPattern {
    pub frequency: DreamTemporalFrequency,
    pub interval: u16,
    pub weekday: Option<DreamTemporalWeekday>,
    pub month_day: Option<u8>,
    pub month: Option<u8>,
    pub evidence: String,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct DreamTemporalAnalysis {
    pub source_timestamp_ns: Option<i64>,
    pub anchors: Vec<DreamTemporalAnchor>,
    pub patterns: Vec<DreamTemporalPattern>,
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum DreamTemporalMatchKind {
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
pub struct DreamTemporalMatch {
    pub kind: DreamTemporalMatchKind,
    pub source_anchor: Option<DreamTemporalAnchor>,
    pub candidate_anchor: Option<DreamTemporalAnchor>,
    pub source_pattern: Option<DreamTemporalPattern>,
    pub candidate_pattern: Option<DreamTemporalPattern>,
}
