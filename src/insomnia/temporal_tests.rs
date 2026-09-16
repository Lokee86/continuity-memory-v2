use super::temporal::assess_draft;
use crate::{MemoryDraft, TemporalIndicationKind, TemporalResolutionStatus};
use time::{Date, Month};

#[test]
fn insomnia_draft_assessment_uses_authoritative_source_time() {
    let source_time_ns = timestamp_ns(2026, Month::September, 16);
    let draft = draft("Review tomorrow.", Some(source_time_ns));
    let assessment = assess_draft(&draft);

    assert_eq!(
        assessment.analysis.source_timestamp_ns,
        Some(source_time_ns)
    );
    assert_eq!(
        assessment.resolution.status,
        TemporalResolutionStatus::FullyResolved
    );
    assert!(assessment.resolution.unresolved_indications.is_empty());
}

#[test]
fn insomnia_draft_assessment_carries_only_unresolved_residue() {
    let source_time_ns = timestamp_ns(2026, Month::September, 16);
    let draft = draft(
        "Deadline 2026-10-01; schedule a biweekly review.",
        Some(source_time_ns),
    );
    let assessment = assess_draft(&draft);

    assert_eq!(
        assessment.resolution.status,
        TemporalResolutionStatus::Unresolved
    );
    assert!(
        assessment
            .resolution
            .unresolved_indications
            .iter()
            .any(|value| {
                value.kind == TemporalIndicationKind::Recurrence
                    && value.evidence.eq_ignore_ascii_case("biweekly")
            })
    );
    assert!(
        !assessment
            .resolution
            .unresolved_indications
            .iter()
            .any(|value| {
                value.kind == TemporalIndicationKind::Explicit && value.evidence == "2026-10-01"
            })
    );
}

fn draft(content: &str, source_time_ns: Option<i64>) -> MemoryDraft {
    MemoryDraft {
        category: "fact".into(),
        memory_type: "project".into(),
        authority_kind: "direct".into(),
        temporal_status: "current".into(),
        title: "Temporal test".into(),
        content: content.into(),
        scope: "test".into(),
        lifecycle_state: "extracted".into(),
        archived: false,
        superseded_by: None,
        parent_id: None,
        source_node_id: None,
        content_source_conversation_id: None,
        content_source_node_id: None,
        grounding_source_conversation_id: None,
        grounding_source_node_id: None,
        source_episode_id: None,
        source_time_ns,
        mutation_id: "insomnia-temporal-test".into(),
        created_at_ns: 1,
        updated_at_ns: 1,
    }
}

fn timestamp_ns(year: i32, month: Month, day: u8) -> i64 {
    let date = Date::from_calendar_date(year, month, day).unwrap();
    i64::try_from(
        date.with_hms(12, 0, 0)
            .unwrap()
            .assume_utc()
            .unix_timestamp_nanos(),
    )
    .unwrap()
}
