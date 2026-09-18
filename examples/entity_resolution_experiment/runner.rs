use reliquary_memory::{ConfiguredGeneralEndpoint, GeneralEndpoint};
use serde_json::{Value, json};
use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, Mutex, mpsc},
    thread,
};

pub fn run(
    endpoint: ConfiguredGeneralEndpoint,
    prior: Vec<Value>,
    corpus: HashMap<String, Value>,
    workers: usize,
) -> Result<Vec<Value>, String> {
    let owners: HashMap<String, String> = corpus
        .iter()
        .filter_map(|(id, v)| Some((id.clone(), v["owner_id"].as_str()?.to_owned())))
        .collect();
    let mut surfaces: HashMap<(String, String), HashSet<String>> = HashMap::new();
    for row in &prior {
        let id = row["memory_id"].as_str().ok_or("memory_id missing")?;
        let owner = owners
            .get(id)
            .ok_or_else(|| format!("candidate missing: {id}"))?;
        for m in filtered(&row["actual_entity_mentions"]) {
            surfaces
                .entry((owner.clone(), norm(m["text"].as_str().unwrap_or(""))))
                .or_default()
                .insert(id.to_owned());
        }
    }
    let ep = Arc::new(endpoint);
    let prior = Arc::new(prior);
    let corpus = Arc::new(corpus);
    let surfaces = Arc::new(surfaces);
    let next = Arc::new(Mutex::new(0));
    let (tx, rx) = mpsc::channel();
    thread::scope(|s| {
        for _ in 0..workers.min(prior.len().max(1)) {
            let (ep, prior, corpus, surfaces, next, tx) = (
                ep.clone(),
                prior.clone(),
                corpus.clone(),
                surfaces.clone(),
                next.clone(),
                tx.clone(),
            );
            s.spawn(move || {
                loop {
                    let i = {
                        let mut n = next.lock().unwrap();
                        if *n >= prior.len() {
                            break;
                        }
                        let i = *n;
                        *n += 1;
                        i
                    };
                    if tx
                        .send((i, case(&ep, &prior[i], &corpus, &surfaces)))
                        .is_err()
                    {
                        break;
                    }
                }
            });
        }
        drop(tx);
    });
    let mut out = vec![None; prior.len()];
    for (i, r) in rx {
        out[i] = Some(r?);
    }
    out.into_iter()
        .map(|x| x.ok_or("worker omitted case".into()))
        .collect()
}
fn case(
    ep: &ConfiguredGeneralEndpoint,
    prior: &Value,
    corpus: &HashMap<String, Value>,
    surfaces: &HashMap<(String, String), HashSet<String>>,
) -> Result<Value, String> {
    let id = prior["memory_id"].as_str().ok_or("memory_id missing")?;
    let mem = corpus
        .get(id)
        .ok_or_else(|| format!("candidate missing: {id}"))?;
    let owner = mem["owner_id"].as_str().ok_or("owner_id missing")?;
    let base = filtered(&prior["actual_entity_mentions"]);
    let mut det = Vec::new();
    let mut remaining = Vec::new();
    for (i, m) in base.iter().enumerate() {
        let surface = norm(m["text"].as_str().unwrap_or(""));
        let method = if surfaces
            .get(&(owner.to_owned(), surface.clone()))
            .map_or(false, |s| s.iter().any(|x| x != id))
        {
            Some("exact_surface")
        } else if surface == "user" || surface == "the user" {
            Some("owner_self")
        } else {
            None
        };
        if let Some(method) = method {
            det.push(json!({"candidate":m,"method":method,"decision":"resolve_existing"}));
        } else {
            remaining.push((i, m.clone()));
        }
    }
    let mut decisions = Vec::new();
    let mut raw = Value::Null;
    let mut error = Value::Null;
    let mut call = false;
    if !remaining.is_empty() {
        call = true;
        let schema = json!({"type":"object","additionalProperties":false,"required":["decisions"],"properties":{"decisions":{"type":"array","items":{"type":"object","additionalProperties":false,"required":["index","decision","reason"],"properties":{"index":{"type":"integer"},"decision":{"type":"string","enum":["create_new","unresolved","reject"]},"reason":{"type":"string","enum":["named_referent","persistent_artifact","descriptive_identity","first_seen_identity","insufficient_evidence","generic_role","abstract_process","transient_value","sentence_local","wrapper_category","ambiguous"]}}}}}});
        let payload = json!({"title":mem["title"],"content":mem["content"],"candidates":remaining.iter().enumerate().map(|(j,(_,m))|json!({"index":j,"field":m["field"],"start_byte":m["start_byte"],"end_byte":m["end_byte"],"text":m["text"]})).collect::<Vec<_>>()});
        match ep.complete_json("Resolve only supplied Entity candidate spans. Return exactly one decision per candidate: create_new only when this mention carries reusable identity even if first-seen; unresolved when plausible but one Memory is insufficient; reject only when it is not a durable Entity candidate. DO NOT reject merely because it is first-seen. DO NOT reject stable owner-relative identities or stable descriptive components just for being descriptive. Reject generic/context-only roles, abstract processes/states/actions, transient values/roles, sentence-local objects, and wrapper/category/activity phrases. Do not add, rewrite, merge, or alter spans.",&payload.to_string(),"entity_resolution_experiment_v1",&schema){Ok(v)=>{raw=v; match validate(&raw,remaining.len()){Ok(v)=>decisions=v,Err(e)=>error=json!(e)}},Err(e)=>return Err(e.to_string())}
    }
    let mut model = json!([]);
    for (j, (i, m)) in remaining.iter().enumerate() {
        let d = decisions
            .get(j)
            .cloned()
            .unwrap_or(json!({"index":i,"decision":"unresolved","reason":"insufficient_evidence"}));
        model
            .as_array_mut()
            .unwrap()
            .push(json!({"candidate":m,"decision":d["decision"],"reason":d["reason"]}));
    }
    let resolved: Vec<Value> = det
        .iter()
        .map(|d| d["candidate"].clone())
        .chain(
            model
                .as_array()
                .unwrap()
                .iter()
                .filter(|d| d["decision"] == "create_new")
                .map(|d| d["candidate"].clone()),
        )
        .collect();
    let preserved: Vec<Value> = det
        .iter()
        .map(|d| d["candidate"].clone())
        .chain(
            model
                .as_array()
                .unwrap()
                .iter()
                .filter(|d| d["decision"] != "reject")
                .map(|d| d["candidate"].clone()),
        )
        .collect();
    let rejected: Vec<Value> = model
        .as_array()
        .unwrap()
        .iter()
        .filter(|d| d["decision"] == "reject")
        .map(|d| d["candidate"].clone())
        .collect();
    Ok(
        json!({"memory_id":id,"owner_id":owner,"expected_entity_mentions":prior["expected_entity_mentions"],"baseline_entity_mentions":base,"deterministic_resolutions":det,"model_decisions":model,"resolved_entity_mentions":resolved,"preserved_entity_mentions":preserved,"rejected_entity_mentions":rejected,"validator_call":false,"model_call":call,"raw":raw,"error":error}),
    )
}
fn validate(raw: &Value, n: usize) -> Result<Vec<Value>, &'static str> {
    let a = raw["decisions"].as_array().ok_or("missing decisions")?;
    if a.len() != n {
        return Err("decision count mismatch");
    }
    let mut seen = vec![false; n];
    for d in a {
        let i = d["index"].as_u64().ok_or("bad index")? as usize;
        if i >= n || seen[i] {
            return Err("index out of range or duplicate");
        }
        seen[i] = true;
    }
    let mut o = vec![Value::Null; n];
    for d in a {
        o[d["index"].as_u64().unwrap() as usize] = d.clone()
    }
    Ok(o)
}
fn norm(s: &str) -> String {
    s.trim().to_ascii_lowercase()
}
fn filtered(v: &Value) -> Vec<Value> {
    v.as_array()
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter(|m| !generic(m["text"].as_str().unwrap_or("")))
        .collect()
}
fn generic(t: &str) -> bool {
    let x = t.trim();
    let l = x.to_ascii_lowercase();
    let heads = [
        "flow",
        "lane",
        "mechanism",
        "request",
        "response",
        "seam",
        "state",
        "status",
    ];
    if x.split_whitespace().count() > 1
        && heads.contains(&l.split_whitespace().last().unwrap_or(""))
    {
        return true;
    }
    const G: &[&str] = &[
        "agent",
        "the agent",
        "application",
        "the application",
        "assistant",
        "the assistant",
        "client",
        "the client",
        "code",
        "the code",
        "data",
        "the data",
        "documentation",
        "the documentation",
        "entry",
        "the entry",
        "file",
        "the file",
        "flow",
        "the flow",
        "implementation",
        "the implementation",
        "lane",
        "the lane",
        "pipeline",
        "the pipeline",
        "project",
        "the project",
        "repository",
        "the repository",
        "request",
        "the request",
        "response",
        "the response",
        "script",
        "the script",
        "scripts",
        "the scripts",
        "server",
        "the server",
        "session",
        "the session",
        "shader",
        "the shader",
        "state",
        "the state",
        "status",
        "the status",
        "system",
        "the system",
        "tooling",
        "the tooling",
        "login status",
        "the login status",
        "requested state",
        "the requested state",
        "relevant repository",
        "the relevant repository",
        "existing server",
        "the existing server",
    ];
    (l.starts_with("the ") || x == l) && G.contains(&l.as_str())
}
