use serde_json::{Value, json};
use std::collections::HashSet;
pub fn summarize(rows: &[Value], model: &str, effort: &str) -> Value {
    let b = metrics(rows, "baseline_entity_mentions");
    let r = metrics(rows, "resolved_entity_mentions");
    let p = metrics(rows, "preserved_entity_mentions");
    let mut c = json!({"owner_self":0,"mentions_with_prior_candidates":0,"mentions_with_multiple_prior_candidates":0,"resolve_existing":0,"create_new":0,"unresolved":0,"reject":0,"calls":0,"errors":0,"lexical_candidate_mentions_resolved_existing":0,"lexical_candidate_mentions_create_new":0,"lexical_candidate_mentions_unresolved":0,"lexical_candidate_mentions_rejected":0});
    for x in rows {
        if x["model_call"].as_bool() == Some(true) {
            c["calls"] = json!(c["calls"].as_u64().unwrap() + 1)
        }
        if !x["error"].is_null() {
            c["errors"] = json!(c["errors"].as_u64().unwrap() + 1)
        }
        c["owner_self"] = json!(
            c["owner_self"].as_u64().unwrap()
                + x["deterministic_resolutions"]
                    .as_array()
                    .map_or(0, |a| a.len() as u64)
        );
        for d in x["model_decisions"].as_array().into_iter().flatten() {
            let k = match d["decision"].as_str() {
                Some("resolve_existing") => "resolve_existing",
                Some("create_new") => "create_new",
                Some("unresolved") => "unresolved",
                Some("reject") => "reject",
                _ => continue,
            };
            c[k] = json!(c[k].as_u64().unwrap() + 1);
            let n = d["prior_candidate_count"].as_u64().unwrap_or(0);
            if n > 0 {
                c["mentions_with_prior_candidates"] =
                    json!(c["mentions_with_prior_candidates"].as_u64().unwrap() + 1);
                let lexical_key = match k {
                    "resolve_existing" => "lexical_candidate_mentions_resolved_existing".to_owned(),
                    "reject" => "lexical_candidate_mentions_rejected".to_owned(),
                    _ => format!("lexical_candidate_mentions_{k}"),
                };
                c[&lexical_key] = json!(c[&lexical_key].as_u64().unwrap() + 1);
                if n > 1 {
                    c["mentions_with_multiple_prior_candidates"] = json!(
                        c["mentions_with_multiple_prior_candidates"]
                            .as_u64()
                            .unwrap()
                            + 1
                    )
                }
            }
        }
    }
    json!({"model":model,"reasoning_effort":effort,"cases":rows.len(),"baseline":b,"resolved_now":r,"preserved":p,"counts":c,"baseline_gold_selectivity":selectivity(rows)})
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
fn metrics(rows: &[Value], k: &str) -> Value {
    let (mut t, mut f, mut n, mut e, mut z, mut za) = (0, 0, 0, 0, 0, 0);
    for r in rows {
        let a = set(&r["expected_entity_mentions"]);
        let b = set(&r[k]);
        t += a.intersection(&b).count();
        f += b.difference(&a).count();
        n += a.difference(&b).count();
        e += usize::from(a == b);
        if a.is_empty() {
            z += 1;
            za += usize::from(b.is_empty())
        }
    }
    let p = ratio(t, t + f);
    let q = ratio(t, t + n);
    json!({"tp":t,"fp":f,"fn":n,"precision":p,"recall":q,"f1":if p+q==0.0{0.0}else{2.0*p*q/(p+q)},"exact":e,"zero":z,"zero_accuracy":ratio(za,z)})
}
fn ratio(a: usize, b: usize) -> f64 {
    if b == 0 { 1.0 } else { a as f64 / b as f64 }
}
fn selectivity(rows: &[Value]) -> Value {
    let mut o = json!({
        "baseline_tp_resolved": 0, "baseline_tp_unresolved": 0,
        "baseline_tp_rejected": 0, "baseline_fp_resolved": 0,
        "baseline_fp_unresolved": 0, "baseline_fp_rejected": 0,
    });
    for r in rows {
        let e = set(&r["expected_entity_mentions"]);
        let mut status = std::collections::HashMap::new();
        for d in r["deterministic_resolutions"]
            .as_array()
            .into_iter()
            .flatten()
        {
            status.insert(key(&d["candidate"]), "resolved");
        }
        for d in r["model_decisions"].as_array().into_iter().flatten() {
            let s = match d["decision"].as_str() {
                Some("resolve_existing") | Some("create_new") => "resolved",
                Some("unresolved") => "unresolved",
                Some("reject") => "rejected",
                _ => continue,
            };
            status.insert(key(&d["candidate"]), s);
        }
        for mention in set(&r["baseline_entity_mentions"]) {
            let class = status.get(&mention).copied().unwrap_or("unresolved");
            let kind = if e.contains(&mention) { "tp" } else { "fp" };
            let field = format!("baseline_{kind}_{class}");
            o[field] = json!(o[&field].as_u64().unwrap_or(0) + 1);
        }
    }
    o
}

fn key(v: &Value) -> String {
    format!(
        "{}:{}:{}:{}",
        v["field"], v["start_byte"], v["end_byte"], v["text"]
    )
}
