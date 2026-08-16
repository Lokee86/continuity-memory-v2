use super::candidate_text::normalize;

pub(super) fn is_pure_execution_receipt(title: &str, content: &str) -> bool {
    let combined = format!(" {} {} ", normalize(title), normalize(content));
    let has_transient_unit = [
        " prompt ",
        " phase ",
        " step ",
        " test run ",
        " tests ",
        " commit ",
        " push ",
        " merge ",
        " verification ",
        " cleanup ",
        " worktree ",
        " branch ",
        " milestone ",
    ]
    .iter()
    .any(|marker| combined.contains(marker));
    let has_completion = [
        " completed ",
        " complete ",
        " done ",
        " finished ",
        " passed ",
        " pushed ",
        " merged ",
        " committed ",
        " verified ",
        " reached ",
        " clean ",
    ]
    .iter()
    .any(|marker| combined.contains(marker));
    has_transient_unit && has_completion && !has_durable_state_signal(content)
}

fn has_durable_state_signal(content: &str) -> bool {
    let padded = format!(" {} ", normalize(content));
    [
        " uses ",
        " owns ",
        " references ",
        " stores ",
        " requires ",
        " configured ",
        " now uses ",
        " moved to ",
        " lives in ",
        " returns ",
        " sends ",
        " persists ",
        " wired through ",
        " routes through ",
        " supports ",
        " rejects ",
        " accepts ",
    ]
    .iter()
    .any(|marker| padded.contains(marker))
}
