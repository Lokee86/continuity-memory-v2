use reliquary_memory::{ConfiguredGeneralEndpoint, GeneralEndpoint};
use serde_json::{Value, json};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex, mpsc},
    thread,
};

pub fn run(
    endpoint: ConfiguredGeneralEndpoint,
    prior: Vec<Value>,
    corpus: HashMap<String, Value>,
    workers: usize,
) -> Result<Vec<Value>, String> {
    let n = prior.len();
    let ep = Arc::new(endpoint);
    let prior = Arc::new(prior);
    let corpus = Arc::new(corpus);
    let next = Arc::new(Mutex::new(0));
    let (tx, rx) = mpsc::channel();
    thread::scope(|s| {
        for _ in 0..workers.min(n.max(1)) {
            let (ep, prior, corpus, next, tx) = (
                ep.clone(),
                prior.clone(),
                corpus.clone(),
                next.clone(),
                tx.clone(),
            );
            s.spawn(move || {
                loop {
                    let i = {
                        let mut x = next.lock().unwrap();
                        if *x >= prior.len() {
                            break;
                        }
                        let i = *x;
                        *x += 1;
                        i
                    };
                    if tx.send((i, case(&ep, &prior[i], &corpus))).is_err() {
                        break;
                    }
                }
            });
        }
        drop(tx);
    });
    let mut out = vec![None; n];
    for (i, r) in rx {
        out[i] = Some(r?);
    }
    out.into_iter()
        .map(|v| v.ok_or("worker omitted case".into()))
        .collect()
}
fn case(
    ep: &ConfiguredGeneralEndpoint,
    prior: &Value,
    corpus: &HashMap<String, Value>,
) -> Result<Value, String> {
    let id = prior["memory_id"].as_str().ok_or("memory_id missing")?;
    let mem = corpus
        .get(id)
        .ok_or_else(|| format!("candidate missing: {id}"))?;
    let baseline = prior["actual_entity_mentions"]
        .as_array()
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter(|m| !generic(m["text"].as_str().unwrap_or("")))
        .collect::<Vec<_>>();
    if baseline.is_empty() {
        return Ok(
            json!({"memory_id":id,"expected_entity_mentions":prior["expected_entity_mentions"],"baseline_entity_mentions":[],"validated_entity_mentions":[],"decisions":[],"raw":null,"error":null,"validator_call":false}),
        );
    }
    let schema = json!({"type":"object","additionalProperties":false,"required":["decisions"],"properties":{"decisions":{"type":"array","items":{"type":"object","additionalProperties":false,"required":["index","decision","reason"],"properties":{"index":{"type":"integer"},"decision":{"type":"string","enum":["keep","reject","uncertain"]},"reason":{"type":"string","enum":["named_referent","persistent_artifact","descriptive_identity","generic_role","abstract_process","transient_value","sentence_local","ambiguous"]}}}}}});
    let payload = json!({"title":mem["title"],"content":mem["content"],"candidates":baseline.iter().enumerate().map(|(i,m)|json!({"index":i,"field":m["field"],"start_byte":m["start_byte"],"end_byte":m["end_byte"],"text":m["text"]})).collect::<Vec<_>>()});
    let raw = ep
        .complete_json(
            "Validate durable Entity identity only. Durable identity means that it is semantically legitimate and useful to assign this mention a stable Entity ID that can be linked from future Memories. First-seen or new entities are allowed. Keep named people, organizations, places, projects, and products; persistent files and paths; code and API artifacts; stable owner-relative identities; and stable descriptive components only when the phrase itself carries reusable identity. Reject generic or context-only roles, abstract state/process/action, category or activity wrappers around a valid referent, transient runtime values or roles, and sentence-local objects. Do not keep a mention merely because it is salient. Uncertain means genuine identity ambiguity and must not create an Entity. Do not add, rewrite, merge, or alter spans.",
            &payload.to_string(),
            "entity_validation_experiment_v1",
            &schema,
        )
        .map_err(|e| e.to_string())?;
    let decisions = match validate(&raw, baseline.len()) {
        Ok(value) => value,
        Err(error) => {
            return Ok(
                json!({"memory_id":id,"expected_entity_mentions":prior["expected_entity_mentions"],"baseline_entity_mentions":baseline,"validated_entity_mentions":[],"decisions":[],"raw":raw,"error":error,"validator_call":true}),
            );
        }
    };
    let kept = baseline
        .iter()
        .enumerate()
        .filter(|(i, _)| decisions[*i]["decision"] == "keep")
        .map(|(_, m)| m.clone())
        .collect::<Vec<_>>();
    Ok(
        json!({"memory_id":id,"expected_entity_mentions":prior["expected_entity_mentions"],"baseline_entity_mentions":baseline,"validated_entity_mentions":kept,"decisions":decisions,"raw":raw,"error":null,"validator_call":true}),
    )
}
fn validate(raw: &Value, n: usize) -> Result<Vec<Value>, &'static str> {
    let a = raw["decisions"].as_array().ok_or("missing decisions")?;
    if a.len() != n {
        return Err("decision count mismatch");
    };
    let mut seen = vec![false; n];
    for d in a {
        let i = d["index"].as_u64().ok_or("bad index")? as usize;
        if i >= n || seen[i] {
            return Err("index out of range or duplicate");
        };
        seen[i] = true;
        if d["decision"].as_str().is_none() || d["reason"].as_str().is_none() {
            return Err("bad decision");
        }
    }
    let mut ordered = vec![Value::Null; n];
    for d in a {
        let i = d["index"].as_u64().ok_or("bad index")? as usize;
        ordered[i] = d.clone();
    }
    Ok(ordered)
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
