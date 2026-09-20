use crate::{
    client::{JevClient, choice},
    common::{norm, row_mentions},
};
use serde_json::{Map, Value, json};
use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, Mutex, mpsc},
    thread,
};

pub fn run(
    client: JevClient,
    prior: Vec<Value>,
    corpus: HashMap<String, Value>,
    workers: usize,
) -> Vec<Value> {
    let owners: HashMap<String, String> = corpus
        .iter()
        .filter_map(|(id, v)| Some((id.clone(), v["owner_id"].as_str()?.to_owned())))
        .collect();
    let mut surfaces: HashMap<(String, String), HashSet<String>> = HashMap::new();
    for row in &prior {
        let Some(id) = row["memory_id"].as_str() else {
            continue;
        };
        let Some(owner) = owners.get(id) else {
            continue;
        };
        for mention in row_mentions(row) {
            surfaces
                .entry((owner.clone(), norm(mention["text"].as_str().unwrap_or(""))))
                .or_default()
                .insert(id.to_owned());
        }
    }

    let client = Arc::new(client);
    let prior = Arc::new(prior);
    let corpus = Arc::new(corpus);
    let surfaces = Arc::new(surfaces);
    let next = Arc::new(Mutex::new(0usize));
    let (tx, rx) = mpsc::channel();

    thread::scope(|scope| {
        for _ in 0..workers.min(prior.len().max(1)) {
            let (client, prior, corpus, surfaces, next, tx) = (
                client.clone(),
                prior.clone(),
                corpus.clone(),
                surfaces.clone(),
                next.clone(),
                tx.clone(),
            );
            scope.spawn(move || {
                loop {
                    let index = {
                        let mut cursor = next.lock().unwrap();
                        if *cursor >= prior.len() {
                            break;
                        }
                        let index = *cursor;
                        *cursor += 1;
                        index
                    };
                    if tx
                        .send((index, run_case(&client, &prior[index], &corpus, &surfaces)))
                        .is_err()
                    {
                        break;
                    }
                }
            });
        }
        drop(tx);
    });

    let mut output = vec![Value::Null; prior.len()];
    for (index, row) in rx {
        output[index] = row;
    }
    output
}

fn run_case(
    client: &JevClient,
    prior: &Value,
    corpus: &HashMap<String, Value>,
    surfaces: &HashMap<(String, String), HashSet<String>>,
) -> Value {
    let id = prior["memory_id"].as_str().unwrap_or("");
    let Some(memory) = corpus.get(id) else {
        return json!({"memory_id":id,"error":"candidate memory missing"});
    };
    let owner = memory["owner_id"].as_str().unwrap_or("");
    let baseline = row_mentions(prior);
    let mut deterministic = Vec::new();
    let mut remaining = Vec::new();
    for mention in &baseline {
        let surface = norm(mention["text"].as_str().unwrap_or(""));
        let method = if surfaces
            .get(&(owner.to_owned(), surface))
            .is_some_and(|ids| ids.iter().any(|other| other != id))
        {
            Some("exact_surface")
        } else if mention["text"].as_str().is_some_and(|v| {
            let v = norm(v);
            v == "user" || v == "the user"
        }) {
            Some("owner_self")
        } else {
            None
        };
        if let Some(method) = method {
            deterministic
                .push(json!({"candidate":mention,"method":method,"decision":"resolve_existing"}));
        } else {
            remaining.push(mention.clone());
        }
    }

    let mut model = Vec::new();
    let mut raw = Value::Null;
    let mut error = Value::Null;
    if !remaining.is_empty() {
        let mut questions = Map::new();
        for (index, _) in remaining.iter().enumerate() {
            questions.insert(format!("m{index}"), json!({
                "type":"choice",
                "instructions":format!(
                    "Classify mention index {index} only. Which disposition is correct for this mention in the supplied Memory?"
                ),
                "criteria":{
                    "create_new":"The mention denotes a durable reusable identity and should be admitted as a new Entity. First-seen is allowed; stable owner-relative and descriptive identities are allowed.",
                    "unresolved":"The mention plausibly denotes a durable reusable identity, but this Memory alone is insufficient or ambiguous, so no terminal decision is safe.",
                    "reject":"The mention is not a durable Entity candidate: it is a generic/context-only role, abstract process/state/action, transient runtime value/role, sentence-local object, or wrapper/category/activity phrase."
                }
            }));
        }
        let state = json!({
            "title":memory["title"],
            "content":memory["content"],
            "mentions":remaining.iter().enumerate().map(|(index, mention)| json!({
                "index":index,"field":mention["field"],"start_byte":mention["start_byte"],
                "end_byte":mention["end_byte"],"text":mention["text"]
            })).collect::<Vec<_>>()
        });
        match client.evaluate(state, Value::Object(questions)) {
            Ok(response) => {
                raw = response.clone();
                for (index, mention) in remaining.iter().enumerate() {
                    match choice(&response, &format!("m{index}")) {
                        Ok((decision, probabilities, confidence))
                            if matches!(
                                decision.as_str(),
                                "create_new" | "unresolved" | "reject"
                            ) =>
                        {
                            model.push(json!({"candidate":mention,"decision":decision,
                                "probabilities":probabilities,"confidence":confidence}));
                        }
                        Ok((decision, _, _)) => {
                            error = json!(format!("unknown Jev decision: {decision}"));
                            model.push(fallback(mention));
                        }
                        Err(e) => {
                            error = json!(e);
                            model.push(fallback(mention));
                        }
                    }
                }
            }
            Err(e) => {
                error = json!(e);
                model.extend(remaining.iter().map(fallback));
            }
        }
    }

    let resolved: Vec<Value> = deterministic
        .iter()
        .map(|d| d["candidate"].clone())
        .chain(
            model
                .iter()
                .filter(|d| d["decision"] == "create_new")
                .map(|d| d["candidate"].clone()),
        )
        .collect();
    let preserved: Vec<Value> = deterministic
        .iter()
        .map(|d| d["candidate"].clone())
        .chain(
            model
                .iter()
                .filter(|d| d["decision"] != "reject")
                .map(|d| d["candidate"].clone()),
        )
        .collect();
    let rejected: Vec<Value> = model
        .iter()
        .filter(|d| d["decision"] == "reject")
        .map(|d| d["candidate"].clone())
        .collect();

    json!({
        "memory_id":id,"owner_id":owner,"expected_entity_mentions":prior["expected_entity_mentions"],
        "baseline_entity_mentions":baseline,"deterministic_resolutions":deterministic,
        "model_decisions":model,"resolved_entity_mentions":resolved,"preserved_entity_mentions":preserved,
        "rejected_entity_mentions":rejected,"validator_call":false,
        "model_call":!remaining.is_empty(),"raw":raw,"error":error
    })
}

fn fallback(mention: &Value) -> Value {
    json!({"candidate":mention,"decision":"unresolved","probabilities":null,"confidence":null})
}
