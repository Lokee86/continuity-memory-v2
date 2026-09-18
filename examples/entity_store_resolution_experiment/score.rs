use serde_json::{Value, json};

pub fn summarize(rows: &[Value], model: &str, effort: &str) -> Value {
    let mut calls = 0usize;
    let mut errors = 0usize;
    let mut invariant = 0usize;
    let mut candidate_total = 0usize;
    let mut permutation = counts();
    let mut consensus = counts();

    for row in rows {
        let results = row["permutation_results"]
            .as_array()
            .unwrap_or(&Vec::new())
            .clone();
        calls += results.len();
        candidate_total += row["base_candidate_entity_ids"]
            .as_array()
            .map_or(0, Vec::len);
        if row["order_invariant"] == true {
            invariant += 1;
            if let Some(decision) = row["consensus"]["decision"].as_str() {
                increment(&mut consensus, decision);
            }
        }
        for result in results {
            if !result["error"].is_null() {
                errors += 1;
            }
            if let Some(decision) = result["decision"].as_str() {
                increment(&mut permutation, decision);
            }
        }
    }

    json!({
        "model": model,
        "reasoning_effort": effort,
        "queries": rows.len(),
        "calls": calls,
        "errors": errors,
        "order_invariant_count": invariant,
        "order_invariant_rate": ratio(invariant, rows.len()),
        "non_invariant_count": rows.len().saturating_sub(invariant),
        "consensus_decision_distribution": consensus,
        "permutation_decision_distribution": permutation,
        "candidate_count_total": candidate_total,
        "candidate_count_average": if rows.is_empty() { 0.0 } else { candidate_total as f64 / rows.len() as f64 },
    })
}

fn counts() -> Value {
    json!({"resolve_existing":0,"create_new":0,"unresolved":0,"reject":0})
}

fn increment(counts: &mut Value, key: &str) {
    if let Some(value) = counts[key].as_u64() {
        counts[key] = json!(value + 1);
    }
}

fn ratio(a: usize, b: usize) -> f64 {
    if b == 0 { 1.0 } else { a as f64 / b as f64 }
}
