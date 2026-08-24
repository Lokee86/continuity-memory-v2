use super::ledger;
use serde_json::Value;

pub const INSOMNIA_EXTRACTOR_CONTRACT_VERSION: &str = "v3-0";
pub const MAX_INSOMNIA_CANDIDATES: usize = 64;

/// Public compatibility alias for the semantic selection prompt.
///
/// Insomnia v3 is two-pass: this prompt owns authority/disposition selection;
/// final Memory wording is handled by the private synthesis contract.
pub const INSOMNIA_SYSTEM_PROMPT: &str = ledger::LEDGER_SYSTEM_PROMPT;

/// Generic introspection schema for the current Insomnia selector contract.
/// Runtime extraction uses a stricter per-Episode schema whose required keys and
/// provenance enums are derived from the actual Episode/evidence turns.
pub fn insomnia_schema() -> Value {
    ledger::generic_schema()
}
