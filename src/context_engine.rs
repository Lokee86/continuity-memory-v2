#[path = "context_engine/compaction.rs"]
mod compaction;
#[path = "context_engine/compaction_budget.rs"]
mod compaction_budget;
#[path = "context_engine/evidence.rs"]
mod evidence;
#[path = "context_engine/model.rs"]
mod model;
#[cfg(test)]
#[path = "context_engine/tests.rs"]
mod tests;

pub use compaction::{
    COMPACTED_CONTEXT_INSTRUCTIONS, MIN_RAW_TAIL_TURNS, checkpoint_applies, plan_compaction,
    plan_session_compaction, request_context, request_messages,
};
pub use compaction_budget::{
    COMPACTION_INPUT_PERCENT, COMPACTION_TARGET_OF_TRIGGER_PERCENT, CompactionError,
    ContextTokenCounter, DEFAULT_COMPACTION_TRIGGER_PERCENT, MAX_SUMMARY_TOKENS,
    RAW_TAIL_OF_TARGET_PERCENT, SUMMARY_OF_TARGET_PERCENT, budget_satisfied, compaction_budget,
    compaction_needed, normalize_summary,
};
pub use evidence::{render_assistant_content, render_assistant_content_from_echo};
pub use model::{
    CompactionBudget, CompactionCheckpoint, CompactionPlan, ContextEvidence, ContextMessage,
    ContextRequest, ContextRole, ContextTurn, ContextView, EvidenceKind,
};
