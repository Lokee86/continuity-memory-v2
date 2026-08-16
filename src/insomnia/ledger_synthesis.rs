use super::candidate::RawCandidate;
use super::candidate_source::required_string;
use super::extraction::InsomniaExtractionError;
use super::ledger::{DispositionLedger, LedgerUnit};
use serde_json::Value;
use std::collections::{HashMap, HashSet};

pub(super) fn parse_synthesis(
    value: &Value,
    ledger: &DispositionLedger,
) -> Result<Vec<RawCandidate>, InsomniaExtractionError> {
    let memories = value
        .get("memories")
        .and_then(Value::as_array)
        .ok_or_else(|| invalid("missing synthesized memories array"))?;
    let by_id: HashMap<_, _> = ledger
        .retained
        .iter()
        .map(|unit| (unit.id.as_str(), unit))
        .collect();
    let mut seen = HashSet::new();
    let mut raw = Vec::with_capacity(memories.len());
    for memory in memories {
        let ids = string_array(memory, "covered_ledger_ids")?;
        if ids.is_empty() {
            return Err(invalid("synthesized Memory covers no ledger ids"));
        }
        let mut units = Vec::with_capacity(ids.len());
        for id in ids {
            if !seen.insert(id.clone()) {
                return Err(invalid("synthesis covered a ledger id more than once"));
            }
            units.push(
                *by_id
                    .get(id.as_str())
                    .ok_or_else(|| invalid("synthesis returned an unknown ledger id"))?,
            );
        }
        validate_merge_group(&units)?;
        raw.push(candidate(memory, &units)?);
    }
    if seen.len() != ledger.retained.len() {
        let missing: Vec<_> = ledger
            .retained
            .iter()
            .filter(|unit| !seen.contains(&unit.id))
            .map(|unit| unit.id.as_str())
            .collect();
        return Err(invalid(format!(
            "synthesis omitted retained ledger ids: {}",
            missing.join(", ")
        )));
    }
    Ok(raw)
}

fn candidate(
    memory: &Value,
    units: &[&LedgerUnit],
) -> Result<RawCandidate, InsomniaExtractionError> {
    let first = units[0];
    let category = required_string(memory, "category")?
        .trim()
        .to_lowercase()
        .replace('-', "_");
    if !units.iter().any(|unit| unit.category == category) {
        return Err(invalid(
            "synthesis selected a category not authorized by its covered units",
        ));
    }
    let title = required_string(memory, "title")?.trim().to_owned();
    let content = required_string(memory, "content")?.trim().to_owned();
    if title.is_empty() || content.is_empty() {
        return Err(invalid("synthesis returned an empty title or content"));
    }
    Ok(RawCandidate {
        authority_kind: first.authority_kind.clone(),
        category,
        memory_type: first.memory_type.clone(),
        title,
        content,
        source_node_id: first.source_node_id.clone(),
        source_quote: covering_quote(units)?,
        authority_source: first.authority_source.clone(),
        grounding_source: first.grounding_source.clone(),
    })
}

fn validate_merge_group(units: &[&LedgerUnit]) -> Result<(), InsomniaExtractionError> {
    let expected = units[0].merge_group();
    if units.iter().any(|unit| unit.merge_group() != expected) {
        return Err(invalid(
            "synthesis combined retained units from incompatible merge groups",
        ));
    }
    Ok(())
}

fn covering_quote(units: &[&LedgerUnit]) -> Result<String, InsomniaExtractionError> {
    if units.len() == 1 {
        return Ok(units[0].source_quote.clone());
    }
    let text = &units[0].source_turn_content;
    let mut start = text.len();
    let mut end = 0;
    for unit in units {
        let offset = text
            .find(&unit.source_quote)
            .ok_or_else(|| invalid("retained quote disappeared from source turn"))?;
        start = start.min(offset);
        end = end.max(offset + unit.source_quote.len());
    }
    text.get(start..end)
        .map(str::to_owned)
        .ok_or_else(|| invalid("failed to build merged source quote"))
}

fn string_array(value: &Value, field: &str) -> Result<Vec<String>, InsomniaExtractionError> {
    value
        .get(field)
        .and_then(Value::as_array)
        .ok_or_else(|| invalid(format!("missing array field {field}")))?
        .iter()
        .map(|item| {
            item.as_str()
                .map(str::to_owned)
                .ok_or_else(|| invalid(format!("non-string value in {field}")))
        })
        .collect()
}

fn invalid(message: impl Into<String>) -> InsomniaExtractionError {
    InsomniaExtractionError::InvalidOutput(message.into())
}
