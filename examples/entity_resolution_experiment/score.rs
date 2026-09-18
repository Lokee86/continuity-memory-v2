use serde_json::{Value, json};
use std::collections::HashSet;

pub fn summarize(rows: &[Value], model: &str, effort: &str) -> Value {
    let baseline = metrics(rows, "baseline_entity_mentions");
    let resolved = metrics(rows, "resolved_entity_mentions");
    let preserved = metrics(rows, "preserved_entity_mentions");
    let mut exact = 0;
    let mut self_resolved = 0;
    let mut create = 0;
    let mut unresolved = 0;
    let mut reject = 0;
    let mut calls = 0;
    let mut errors = 0;
    for r in rows {
        calls += usize::from(r["model_call"].as_bool().unwrap_or(false));
        errors += usize::from(!r["error"].is_null());
        for d in r["deterministic_resolutions"]
            .as_array()
            .into_iter()
            .flatten()
        {
            if d["method"] == "exact_surface" {
                exact += 1
            } else {
                self_resolved += 1
            }
        }
        for d in r["model_decisions"].as_array().into_iter().flatten() {
            match d["decision"].as_str() {
                Some("create_new") => create += 1,
                Some("unresolved") => unresolved += 1,
                Some("reject") => reject += 1,
                _ => {}
            }
        }
    }
    let mut selectivity = json!({"baseline_tp_resolved":0,"baseline_tp_unresolved":0,"baseline_tp_rejected":0,"baseline_fp_resolved":0,"baseline_fp_unresolved":0,"baseline_fp_rejected":0});
    for r in rows {
        let e = set(&r["expected_entity_mentions"]);
        let b = set(&r["baseline_entity_mentions"]);
        let rr = set(&r["resolved_entity_mentions"]);
        let p = set(&r["preserved_entity_mentions"]);
        let x = set(&r["rejected_entity_mentions"]);
        for k in b.intersection(&e) {
            if rr.contains(k) {
                selectivity["baseline_tp_resolved"] =
                    json!(selectivity["baseline_tp_resolved"].as_u64().unwrap() + 1)
            } else if p.contains(k) {
                selectivity["baseline_tp_unresolved"] =
                    json!(selectivity["baseline_tp_unresolved"].as_u64().unwrap() + 1)
            } else if x.contains(k) {
                selectivity["baseline_tp_rejected"] =
                    json!(selectivity["baseline_tp_rejected"].as_u64().unwrap() + 1)
            }
        }
        for k in b.difference(&e) {
            if rr.contains(k) {
                selectivity["baseline_fp_resolved"] =
                    json!(selectivity["baseline_fp_resolved"].as_u64().unwrap() + 1)
            } else if p.contains(k) {
                selectivity["baseline_fp_unresolved"] =
                    json!(selectivity["baseline_fp_unresolved"].as_u64().unwrap() + 1)
            } else if x.contains(k) {
                selectivity["baseline_fp_rejected"] =
                    json!(selectivity["baseline_fp_rejected"].as_u64().unwrap() + 1)
            }
        }
    }
    json!({"model":model,"reasoning_effort":effort,"cases":rows.len(),"baseline":baseline,"resolved_now":resolved,"preserved":preserved,"counts":{"deterministic_resolved_total":exact+self_resolved,"exact_surface":exact,"owner_self":self_resolved,"create_new":create,"unresolved":unresolved,"reject":reject,"calls":calls,"errors":errors},"selectivity":selectivity})
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
    let rec = ratio(tp, tp + fn_);
    json!({"tp":tp,"fp":fp,"fn":fn_,"precision":p,"recall":rec,"f1":if p+rec==0.0{0.0}else{2.0*p*rec/(p+rec)},"exact":exact,"exact_accuracy":ratio(exact,rows.len()),"zero":zero,"zero_accuracy":ratio(zero_ok,zero)})
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
