use reliquary_memory::{DreamMemoryContext, DreamPairClassification, MemoryId};
use serde_json::{Value, json};
use std::collections::BTreeMap;

#[derive(Clone)]
pub struct CaseTask {
    pub id: String,
    pub pattern: String,
    pub confidence: String,
    pub expected: String,
    pub left: DreamMemoryContext,
    pub right: DreamMemoryContext,
}

pub fn memory_id(value: &str) -> Result<MemoryId, String> {
    if value.len() != 64 {
        return Err(format!("invalid MemoryId width: {value}"));
    }
    let mut bytes = [0_u8; 32];
    for (index, byte) in bytes.iter_mut().enumerate() {
        let offset = index * 2;
        *byte = u8::from_str_radix(&value[offset..offset + 2], 16)
            .map_err(|_| format!("invalid MemoryId hex: {value}"))?;
    }
    Ok(MemoryId(bytes))
}

pub fn classification_json(classification: &DreamPairClassification) -> Value {
    json!({
        "relation": format!("{:?}", classification.relation),
        "direction": format!("{:?}", classification.direction),
        "evidence": classification.evidence.iter().map(|evidence| json!({
            "side": format!("{:?}", evidence.side),
            "quote": evidence.quote,
        })).collect::<Vec<_>>(),
    })
}

pub fn summarize(results: &[Value]) -> Value {
    let mut correct = 0usize;
    let mut related_correct = 0usize;
    let mut related_total = 0usize;
    let mut unrelated_correct = 0usize;
    let mut unrelated_total = 0usize;
    let mut errors = 0usize;
    let mut patterns: BTreeMap<String, (usize, usize)> = BTreeMap::new();

    for result in results {
        let expected = result["expected"].as_str().unwrap_or_default();
        let ok = result["correct"].as_bool().unwrap_or(false);
        let has_error = !result["error"].is_null();
        let pattern = result["pattern"].as_str().unwrap_or("unknown").to_owned();
        let entry = patterns.entry(pattern).or_default();
        entry.1 += 1;
        if ok {
            correct += 1;
            entry.0 += 1;
        }
        if has_error {
            errors += 1;
        }
        match expected {
            "related" => {
                related_total += 1;
                related_correct += usize::from(ok);
            }
            "unrelated" => {
                unrelated_total += 1;
                unrelated_correct += usize::from(ok);
            }
            _ => {}
        }
    }

    json!({
        "correct": correct,
        "total": results.len(),
        "related_correct": related_correct,
        "related_total": related_total,
        "unrelated_correct": unrelated_correct,
        "unrelated_total": unrelated_total,
        "errors": errors,
        "patterns": patterns.into_iter().map(|(pattern, (good, total))| {
            (pattern, json!({"correct": good, "total": total}))
        }).collect::<serde_json::Map<_, _>>(),
    })
}
