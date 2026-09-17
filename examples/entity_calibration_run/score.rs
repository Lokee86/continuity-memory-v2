use serde_json::{Value, json};
use std::collections::HashSet;

pub fn summarize(results: &[Value], model: &str, effort: &str) -> Value {
    let mut tp = 0usize;
    let mut fp = 0usize;
    let mut fn_count = 0usize;
    let mut exact_cases = 0usize;
    let mut zero_total = 0usize;
    let mut zero_correct = 0usize;
    let mut errors = 0usize;
    let mut required_terms = 0usize;
    let mut required_hits = 0usize;
    let mut actual_terms = 0usize;
    let mut acceptable_hits = 0usize;
    let mut unreviewed_terms = 0usize;

    for row in results {
        let expected = mention_set(&row["expected_entity_mentions"]);
        let actual = mention_set(&row["actual_entity_mentions"]);
        let failed = !row["error"].is_null();
        if failed {
            errors += 1;
        }
        let local_tp = expected.intersection(&actual).count();
        tp += local_tp;
        fp += actual.difference(&expected).count();
        fn_count += expected.difference(&actual).count();
        if !failed && expected == actual {
            exact_cases += 1;
        }
        if expected.is_empty() {
            zero_total += 1;
            if !failed && actual.is_empty() {
                zero_correct += 1;
            }
        }

        let required = string_set(&row["required_lexical_terms"]);
        let acceptable = string_set(&row["acceptable_lexical_terms"]);
        let produced = string_set(&row["actual_lexical_terms"]);
        required_terms += required.len();
        required_hits += required.intersection(&produced).count();
        actual_terms += produced.len();
        acceptable_hits += acceptable.intersection(&produced).count();
        unreviewed_terms += produced.difference(&acceptable).count();
    }

    let precision = ratio(tp, tp + fp);
    let recall = ratio(tp, tp + fn_count);
    let f1 = if precision + recall == 0.0 {
        0.0
    } else {
        2.0 * precision * recall / (precision + recall)
    };
    json!({
        "model": model,
        "reasoning_effort": effort,
        "cases": results.len(),
        "errors": errors,
        "entities": {
            "tp": tp, "fp": fp, "fn": fn_count,
            "precision": precision, "recall": recall, "f1": f1,
            "exact_cases": exact_cases,
            "exact_case_rate": ratio(exact_cases, results.len()),
            "zero_entity_cases": zero_total,
            "zero_entity_correct": zero_correct,
            "zero_entity_accuracy": ratio(zero_correct, zero_total)
        },
        "lexical_terms": {
            "required": required_terms,
            "required_hits": required_hits,
            "required_recall": ratio(required_hits, required_terms),
            "actual_terms": actual_terms,
            "acceptable_hits": acceptable_hits,
            "unreviewed_terms": unreviewed_terms
        }
    })
}

fn mention_set(value: &Value) -> HashSet<String> {
    value
        .as_array()
        .into_iter()
        .flatten()
        .map(|item| {
            format!(
                "{}:{}:{}:{}",
                item["field"].as_str().unwrap_or(""),
                item["start_byte"].as_u64().unwrap_or(u64::MAX),
                item["end_byte"].as_u64().unwrap_or(u64::MAX),
                item["text"].as_str().unwrap_or("")
            )
        })
        .collect()
}

fn string_set(value: &Value) -> HashSet<String> {
    value
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(str::to_owned)
        .collect()
}

fn ratio(part: usize, total: usize) -> f64 {
    if total == 0 {
        1.0
    } else {
        part as f64 / total as f64
    }
}
