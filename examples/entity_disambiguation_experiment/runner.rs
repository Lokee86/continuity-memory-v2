use reliquary_memory::{ConfiguredGeneralEndpoint, GeneralEndpoint};
use serde_json::{Value, json};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex, mpsc},
    thread,
};
const REASONS: &[&str] = &[
    "context_match",
    "context_conflict_new_identity",
    "named_referent",
    "persistent_artifact",
    "descriptive_identity",
    "first_seen_identity",
    "insufficient_evidence",
    "generic_role",
    "abstract_process",
    "transient_value",
    "sentence_local",
    "wrapper_category",
    "ambiguous",
];
pub fn run(
    ep: ConfiguredGeneralEndpoint,
    prior: Vec<Value>,
    corpus: HashMap<String, Value>,
    workers: usize,
) -> Result<Vec<Value>, String> {
    let mut anchors: HashMap<(String, String), Vec<Value>> = HashMap::new();
    for row in &prior {
        let id = row["memory_id"].as_str().ok_or("memory_id missing")?;
        let mem = corpus.get(id).ok_or("memory missing")?;
        let owner = mem["owner_id"].as_str().ok_or("owner_id missing")?;
        for mention in filtered(&row["actual_entity_mentions"]) {
            let surface = norm(mention["text"].as_str().unwrap_or(""));
            let entries = anchors.entry((owner.to_owned(), surface)).or_default();
            if !entries.iter().any(|x| x["memory_id"] == id) {
                entries.push(json!({"memory_id":id,"title":mem["title"],"content":mem["content"],"prior_mention_surface":mention["text"]}));
            }
        }
    }
    for entries in anchors.values_mut() {
        entries.sort_by(|a, b| a["memory_id"].as_str().cmp(&b["memory_id"].as_str()));
    }
    let anchors = Arc::new(anchors);
    let ep = Arc::new(ep);
    let p = Arc::new(prior);
    let c = Arc::new(corpus);
    let next = Arc::new(Mutex::new(0));
    let (tx, rx) = mpsc::channel();
    thread::scope(|s| {
        for _ in 0..workers.min(p.len().max(1)) {
            let (ep, p, c, anchors, next, tx) = (
                ep.clone(),
                p.clone(),
                c.clone(),
                anchors.clone(),
                next.clone(),
                tx.clone(),
            );
            s.spawn(move || {
                loop {
                    let i = {
                        let mut n = next.lock().unwrap();
                        if *n >= p.len() {
                            break;
                        }
                        let i = *n;
                        *n += 1;
                        i
                    };
                    if tx.send((i, case(&ep, &p[i], &c, &anchors))).is_err() {
                        break;
                    }
                }
            });
        }
        drop(tx)
    });
    let mut out = vec![None; p.len()];
    for (i, r) in rx {
        out[i] = Some(r?)
    }
    out.into_iter()
        .map(|x| x.ok_or("worker omitted case".into()))
        .collect()
}
fn case(
    ep: &ConfiguredGeneralEndpoint,
    row: &Value,
    corpus: &HashMap<String, Value>,
    anchors: &HashMap<(String, String), Vec<Value>>,
) -> Result<Value, String> {
    let id = row["memory_id"].as_str().ok_or("memory_id missing")?;
    let mem = corpus.get(id).ok_or("memory missing")?;
    let owner = mem["owner_id"].as_str().ok_or("owner_id missing")?;
    let base = filtered(&row["actual_entity_mentions"]);
    let mut det = Vec::new();
    let mut rem = Vec::new();
    for (i, m) in base.iter().enumerate() {
        let s = norm(m["text"].as_str().unwrap_or(""));
        if s == "user" || s == "the user" {
            det.push(json!({"candidate":m,"method":"owner_self","decision":"resolve_existing"}));
        } else {
            rem.push((i, m.clone()));
        }
    }
    let mut pc = Vec::new();
    for (_, m) in &rem {
        let s = norm(m["text"].as_str().unwrap_or(""));
        pc.push(
            anchors
                .get(&(owner.to_owned(), s))
                .into_iter()
                .flatten()
                .filter(|v| v["memory_id"].as_str() != Some(id))
                .take(6)
                .enumerate()
                .map(|(j, v)| {
                    let mut x = v.clone();
                    x["candidate_index"] = json!(j);
                    x
                })
                .collect(),
        );
    }
    let mut raw = Value::Null;
    let mut error = Value::Null;
    let mut ds = Vec::new();
    let call = !rem.is_empty();
    if call {
        let schema = json!({"type":"object","additionalProperties":false,"required":["decisions"],"properties":{"decisions":{"type":"array","items":{"type":"object","additionalProperties":false,"required":["mention_index","decision","reason","target_candidate_index"],"properties":{"mention_index":{"type":"integer"},"decision":{"type":"string","enum":["resolve_existing","create_new","unresolved","reject"]},"reason":{"type":"string","enum":REASONS},"target_candidate_index":{"type":"integer"}}}}}});
        let payload = json!({"title":mem["title"],"content":mem["content"],"mentions":rem.iter().enumerate().map(|(j,(_,m))|json!({"mention_index":j,"span":m,"prior_candidates":pc[j]})).collect::<Vec<_>>()});
        match ep.complete_json("Lexical equality is NOT identity equality. Do not choose resolve_existing merely because spelling matches. Compare current Memory context against candidate Memory contexts; if the same surface denotes a different referent choose create_new, and if you cannot tell choose unresolved. First-seen can be create_new. Stable owner-relative and descriptive identities are allowed. Reject only when the mention is not a durable Entity candidate at all: a generic or context-only role, abstract process/state/action, transient runtime value/role, sentence-local object, or wrapper/category/activity phrase. Return exactly one decision per mention. Do not add, rewrite, merge, or change spans.",&payload.to_string(),"entity_disambiguation_experiment_v1",&schema){Ok(v)=>{raw=v;match validate(&raw,rem.len(),&pc){Ok(x)=>ds=x,Err(e)=>error=json!(e)}},Err(e)=>return Err(e.to_string())}
    }
    let mut model = Vec::new();
    for (j, (i, m)) in rem.iter().enumerate() {
        let d=ds.get(j).cloned().unwrap_or(json!({"mention_index":j,"decision":"unresolved","reason":"insufficient_evidence","target_candidate_index":-1}));
        let target = if d["decision"] == "resolve_existing" {
            d["target_candidate_index"]
                .as_i64()
                .and_then(|k| usize::try_from(k).ok())
                .and_then(|k| pc[j].get(k).cloned())
        } else {
            None
        };
        model.push(json!({"candidate":m,"mention_index":i,"current_mention":m,"decision":d["decision"],"reason":d["reason"],"target_candidate":target,"prior_candidate_count":pc[j].len()}));
    }
    let resolved = det
        .iter()
        .map(|x| x["candidate"].clone())
        .chain(
            model
                .iter()
                .filter(|x| x["decision"] == "resolve_existing" || x["decision"] == "create_new")
                .map(|x| x["candidate"].clone()),
        )
        .collect::<Vec<_>>();
    let preserved: Vec<Value> = resolved
        .iter()
        .cloned()
        .chain(
            model
                .iter()
                .filter(|x| x["decision"] == "unresolved")
                .map(|x| x["candidate"].clone()),
        )
        .collect();
    let rejected: Vec<Value> = model
        .iter()
        .filter(|x| x["decision"] == "reject")
        .map(|x| x["candidate"].clone())
        .collect();
    Ok(
        json!({"memory_id":id,"owner_id":owner,"expected_entity_mentions":row["expected_entity_mentions"],"baseline_entity_mentions":base,"deterministic_resolutions":det,"model_decisions":model,"resolved_entity_mentions":resolved,"preserved_entity_mentions":preserved,"rejected_entity_mentions":rejected,"model_call":call,"raw":raw,"error":error}),
    )
}
fn validate(v: &Value, n: usize, pc: &[Vec<Value>]) -> Result<Vec<Value>, &'static str> {
    let a = v["decisions"].as_array().ok_or("missing decisions")?;
    if a.len() != n {
        return Err("decision count mismatch");
    }
    let mut out = vec![Value::Null; n];
    for d in a {
        let i = d["mention_index"].as_u64().ok_or("bad mention_index")? as usize;
        if i >= n || !out[i].is_null() {
            return Err("index out of range or duplicate");
        }
        let k = d["target_candidate_index"].as_i64().ok_or("bad target")?;
        let dec = d["decision"].as_str().ok_or("bad decision")?;
        if !REASONS.contains(&d["reason"].as_str().ok_or("bad reason")?)
            || (dec == "resolve_existing" && (k < 0 || k as usize >= pc[i].len()))
        {
            return Err("invalid decision");
        }
        out[i] = d.clone()
    }
    if out.iter().any(Value::is_null) {
        Err("missing decision")
    } else {
        Ok(out)
    }
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
    let h = [
        "flow",
        "lane",
        "mechanism",
        "request",
        "response",
        "seam",
        "state",
        "status",
    ];
    if x.split_whitespace().count() > 1 && h.contains(&l.split_whitespace().last().unwrap_or("")) {
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
