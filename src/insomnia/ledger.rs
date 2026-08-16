use super::candidate_source::{
    RawSource, parse_source, required_string, source_complete, trim_source,
};
use super::contract::MAX_INSOMNIA_CANDIDATES;
use super::extraction::InsomniaExtractionError;
use crate::ResolvedTurn;
use serde_json::Value;

#[derive(Clone)]
pub(super) struct LedgerUnit {
    pub(super) id: String,
    pub(super) source_node_id: String,
    pub(super) source_quote: String,
    pub(super) source_turn_content: String,
    pub(super) authority_kind: String,
    pub(super) category: String,
    pub(super) memory_type: String,
    pub(super) lifecycle: String,
    pub(super) proposition: String,
    pub(super) authority_source: RawSource,
    pub(super) grounding_source: RawSource,
}

pub(super) struct DispositionLedger {
    pub(super) retained: Vec<LedgerUnit>,
}

impl LedgerUnit {
    pub(super) fn merge_group(&self) -> String {
        format!(
            "{}|{}|{}|{}|{}|{}|{}|{}|{}|{}",
            self.source_node_id,
            self.authority_kind,
            self.memory_type,
            self.lifecycle,
            self.authority_source.conversation_id,
            self.authority_source.node_id,
            self.authority_source.quote,
            self.grounding_source.conversation_id,
            self.grounding_source.node_id,
            self.grounding_source.quote,
        )
    }
}

pub(super) fn parse_ledger(
    value: &Value,
    episode_turns: &[ResolvedTurn],
) -> Result<DispositionLedger, InsomniaExtractionError> {
    let turns = value
        .get("turns")
        .and_then(Value::as_array)
        .ok_or_else(|| invalid("missing turns array"))?;
    let user_turns: Vec<_> = episode_turns
        .iter()
        .filter(|turn| turn.role == "user")
        .collect();
    if turns.len() != user_turns.len() {
        return Err(invalid(format!(
            "ledger must account for every user turn: expected {}, got {}",
            user_turns.len(),
            turns.len()
        )));
    }
    let mut retained = Vec::new();
    for (turn_index, (value, source)) in turns.iter().zip(user_turns).enumerate() {
        let source_node_id = required_string(value, "source_node_id")?;
        if source_node_id.trim() != source.node_id {
            return Err(invalid(format!(
                "ledger user turn {turn_index} is out of order or references the wrong node"
            )));
        }
        let units = value
            .get("units")
            .and_then(Value::as_array)
            .filter(|units| !units.is_empty())
            .ok_or_else(|| invalid("every ledger user turn must contain at least one unit"))?;
        for (unit_index, unit) in units.iter().enumerate() {
            if let Some(retained_unit) = parse_unit(unit, source, unit_index)? {
                retained.push(retained_unit);
            }
        }
    }
    if retained.len() > MAX_INSOMNIA_CANDIDATES {
        return Err(invalid(format!(
            "{} retained ledger units exceeds maximum {}",
            retained.len(),
            MAX_INSOMNIA_CANDIDATES
        )));
    }
    Ok(DispositionLedger { retained })
}

fn parse_unit(
    value: &Value,
    turn: &ResolvedTurn,
    unit_index: usize,
) -> Result<Option<LedgerUnit>, InsomniaExtractionError> {
    let source_quote = required_string(value, "source_quote")?.trim().to_owned();
    if source_quote.is_empty() || !turn.content.contains(&source_quote) {
        return Err(invalid(
            "ledger source quote is not verbatim from its user turn",
        ));
    }
    let disposition = label(value, "disposition")?;
    let authority_kind = label(value, "authority_kind")?;
    let category = label(value, "category")?;
    let memory_type = label(value, "type")?;
    let lifecycle = label(value, "lifecycle")?;
    let proposition = required_string(value, "proposition")?.trim().to_owned();
    let _reason = required_string(value, "reason")?;
    let authority_source = trim_source(&parse_source(value, "authority_source")?);
    let grounding_source = trim_source(&parse_source(value, "grounding_source")?);
    if !source_complete(&authority_source) || !source_complete(&grounding_source) {
        return Err(invalid(
            "ledger support-source fields must be all empty or all present",
        ));
    }
    match disposition.as_str() {
        "omit" => {
            if authority_kind != "none"
                || category != "none"
                || memory_type != "none"
                || !proposition.is_empty()
                || !authority_source.node_id.is_empty()
                || !grounding_source.node_id.is_empty()
            {
                return Err(invalid(
                    "omitted ledger units must carry no proposition, authority, type, or support source",
                ));
            }
            Ok(None)
        }
        "retain" => {
            validate_retained_labels(&authority_kind, &category, &memory_type, &lifecycle)?;
            if proposition.is_empty() {
                return Err(invalid("retained ledger unit requires a proposition"));
            }
            Ok(Some(LedgerUnit {
                id: format!("{}:{unit_index}", turn.node_id),
                source_node_id: turn.node_id.clone(),
                source_quote,
                source_turn_content: turn.content.clone(),
                authority_kind,
                category,
                memory_type,
                lifecycle,
                proposition,
                authority_source,
                grounding_source,
            }))
        }
        _ => Err(invalid("ledger disposition is not recognized")),
    }
}

fn validate_retained_labels(
    authority: &str,
    category: &str,
    memory_type: &str,
    lifecycle: &str,
) -> Result<(), InsomniaExtractionError> {
    if !["direct", "correction", "adoption", "retention"].contains(&authority)
        || category == "none"
        || memory_type == "none"
        || !["standing", "current", "planned", "unresolved", "historical"].contains(&lifecycle)
    {
        return Err(invalid(
            "retained ledger unit has invalid authority/category/type/lifecycle",
        ));
    }
    Ok(())
}

fn label(value: &Value, field: &str) -> Result<String, InsomniaExtractionError> {
    Ok(required_string(value, field)?
        .trim()
        .to_lowercase()
        .replace('-', "_"))
}

fn invalid(message: impl Into<String>) -> InsomniaExtractionError {
    InsomniaExtractionError::InvalidOutput(message.into())
}
