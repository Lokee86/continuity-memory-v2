use crate::{MemoryDraft, TemporalAssessment};

pub(super) fn assess_draft(draft: &MemoryDraft) -> TemporalAssessment {
    crate::chronos::assess(
        &format!("{}\n{}", draft.title, draft.content),
        draft.source_time_ns,
    )
}
