use crate::EntityResolutionReason;
use crate::entity_resolver::parse_reason;

#[test]
fn parses_recurrence_required_reason() {
    assert_eq!(
        parse_reason("recurrence_required").unwrap(),
        EntityResolutionReason::RecurrenceRequired
    );
}
