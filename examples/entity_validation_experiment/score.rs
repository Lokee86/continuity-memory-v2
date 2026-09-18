use serde_json::{Value, json};
use std::collections::HashSet;

pub fn summarize(rows: &[Value], model: &str, effort: &str) -> Value {
    let baseline = metrics(rows, "baseline_entity_mentions");
    let validated = metrics(rows, "validated_entity_mentions");
    let mut keep = 0;
    let mut reject = 0;
    let mut uncertain = 0;
    let mut calls = 0;
    let mut errors = 0;
    for row in rows {
        calls += usize::from(row["validator_call"].as_bool().unwrap_or(false));
        if !row["error"].is_null() {
            errors += 1;
        }
        for d in row["decisions"].as_array().into_iter().flatten() {
            match d["decision"].as_str() {
                Some("keep") => keep += 1,
                Some("reject") => reject += 1,
                Some("uncertain") => uncertain += 1,
                _ => {}
            }
        }
    }
    let removed_fp = baseline["fp"]
        .as_u64()
        .unwrap_or(0)
        .saturating_sub(validated["fp"].as_u64().unwrap_or(0));
    let removed_tp = baseline["tp"]
        .as_u64()
        .unwrap_or(0)
        .saturating_sub(validated["tp"].as_u64().unwrap_or(0));
    json!({"model":model,"reasoning_effort":effort,"cases":rows.len(),"baseline":baseline,"validated":validated,"deltas":{"precision":validated["precision"].as_f64().unwrap_or(0.0)-baseline["precision"].as_f64().unwrap_or(0.0),"recall":validated["recall"].as_f64().unwrap_or(0.0)-baseline["recall"].as_f64().unwrap_or(0.0),"f1":validated["f1"].as_f64().unwrap_or(0.0)-baseline["f1"].as_f64().unwrap_or(0.0)},"validator_selectivity":{"removed_baseline_false_positives":removed_fp,"removed_baseline_true_positives":removed_tp,"keep":keep,"reject":reject,"uncertain":uncertain,"calls":calls,"errors":errors}})
}
fn metrics(rows: &[Value], key: &str) -> Value {
    let mut tp = 0;
    let mut fp = 0;
    let mut fn_ = 0;
    let mut exact = 0;
    let mut zero = 0;
    let mut zero_ok = 0;
    for r in rows {
        let e = set(&r["expected_entity_mentions"]);
        let a = set(&r[key]);
        tp += e.intersection(&a).count();
        fp += a.difference(&e).count();
        fn_ += e.difference(&a).count();
        if e == a {
            exact += 1
        }
        if e.is_empty() {
            zero += 1;
            if a.is_empty() {
                zero_ok += 1
            }
        }
    }
    let p = ratio(tp, tp + fp);
    let recall = ratio(tp, tp + fn_);
    json!({"tp":tp,"fp":fp,"fn":fn_,"precision":p,"recall":recall,"f1":if p+recall==0.0{0.0}else{2.0*p*recall/(p+recall)},"exact":exact,"exact_accuracy":ratio(exact,rows.len()),"zero":zero,"zero_accuracy":ratio(zero_ok,zero)})
}
fn set(v: &Value) -> HashSet<String> {
    v.as_array()
        .into_iter()
        .flatten()
        .map(|x| {
            format!(
                "{}:{}:{}:{}",
                x["field"], x["start_byte"], x["end_byte"], x["text"]
            )
        })
        .collect()
}
fn ratio(a: usize, b: usize) -> f64 {
    if b == 0 { 1.0 } else { a as f64 / b as f64 }
}
