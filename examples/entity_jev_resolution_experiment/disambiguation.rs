use crate::{
    client::{JevClient, choice},
    common::{norm, row_mentions},
};
use serde_json::{Map, Value, json};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex, mpsc},
    thread,
};

pub fn run(
    client: JevClient,
    prior: Vec<Value>,
    corpus: HashMap<String, Value>,
    workers: usize,
) -> Vec<Value> {
    let mut anchors: HashMap<(String, String), Vec<Value>> = HashMap::new();
    for row in &prior {
        let Some(id) = row["memory_id"].as_str() else {
            continue;
        };
        let Some(memory) = corpus.get(id) else {
            continue;
        };
        let owner = memory["owner_id"].as_str().unwrap_or("");
        for mention in row_mentions(row) {
            let key = (
                owner.to_owned(),
                norm(mention["text"].as_str().unwrap_or("")),
            );
            let entries = anchors.entry(key).or_default();
            if !entries.iter().any(|v| v["memory_id"].as_str() == Some(id)) {
                entries.push(
                    json!({"memory_id":id,"title":memory["title"],"content":memory["content"],
                    "prior_mention_surface":mention["text"]}),
                );
            }
        }
    }
    for entries in anchors.values_mut() {
        entries.sort_by(|a, b| a["memory_id"].as_str().cmp(&b["memory_id"].as_str()));
    }

    let client = Arc::new(client);
    let prior = Arc::new(prior);
    let corpus = Arc::new(corpus);
    let anchors = Arc::new(anchors);
    let next = Arc::new(Mutex::new(0usize));
    let (tx, rx) = mpsc::channel();
    thread::scope(|scope| {
        for _ in 0..workers.min(prior.len().max(1)) {
            let (client, prior, corpus, anchors, next, tx) = (
                client.clone(),
                prior.clone(),
                corpus.clone(),
                anchors.clone(),
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
                        .send((index, run_case(&client, &prior[index], &corpus, &anchors)))
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
    row: &Value,
    corpus: &HashMap<String, Value>,
    anchors: &HashMap<(String, String), Vec<Value>>,
) -> Value {
    let id = row["memory_id"].as_str().unwrap_or("");
    let Some(memory) = corpus.get(id) else {
        return json!({"memory_id":id,"error":"candidate memory missing"});
    };
    let owner = memory["owner_id"].as_str().unwrap_or("");
    let baseline = row_mentions(row);
    let mut deterministic = Vec::new();
    let mut remaining = Vec::new();
    for mention in &baseline {
        let surface = norm(mention["text"].as_str().unwrap_or(""));
        if surface == "user" || surface == "the user" {
            deterministic.push(
                json!({"candidate":mention,"method":"owner_self","decision":"resolve_existing"}),
            );
        } else {
            remaining.push(mention.clone());
        }
    }

    let prior_candidates: Vec<Vec<Value>> = remaining
        .iter()
        .map(|mention| {
            anchors
                .get(&(
                    owner.to_owned(),
                    norm(mention["text"].as_str().unwrap_or("")),
                ))
                .into_iter()
                .flatten()
                .filter(|candidate| candidate["memory_id"].as_str() != Some(id))
                .take(6)
                .enumerate()
                .map(|(index, candidate)| {
                    let mut value = candidate.clone();
                    value["candidate_index"] = json!(index);
                    value
                })
                .collect()
        })
        .collect();

    let mut questions = Map::new();
    for (index, candidates) in prior_candidates.iter().enumerate() {
        let mut criteria = Map::new();
        for (candidate_index, candidate) in candidates.iter().enumerate() {
            criteria.insert(format!("candidate_{candidate_index}"), json!(format!(
                "Same durable identity as prior candidate {candidate_index}: surface {:?}, title {:?}, context {:?}",
                candidate["prior_mention_surface"].as_str().unwrap_or(""),
                candidate["title"].as_str().unwrap_or(""),
                truncate(candidate["content"].as_str().unwrap_or(""), 1200)
            )));
        }
        criteria.insert("create_new".into(), json!("A durable reusable identity, but none of the prior candidates is the same identity. First-seen or a context-conflicting new identity is allowed."));
        criteria.insert(
            "unresolved".into(),
            json!(
                "Potentially durable, but evidence is insufficient or ambiguous between identities."
            ),
        );
        criteria.insert("reject".into(), json!("Not a durable Entity candidate: generic/context-only role, abstract process/state/action, transient value/role, sentence-local object, or wrapper/category/activity phrase."));
        questions.insert(format!("m{index}"), json!({
            "type":"choice",
            "instructions":format!("Resolve mention index {index}. Lexical equality is not identity equality. Choose a prior candidate only when Memory context supports the same durable identity."),
            "criteria":criteria
        }));
    }

    let state = json!({
        "title":memory["title"],"content":memory["content"],
        "mentions":remaining.iter().enumerate().map(|(index, mention)| json!({
            "index":index,"span":mention,"prior_candidates":prior_candidates[index]
        })).collect::<Vec<_>>()
    });
    let mut raw = Value::Null;
    let mut error = Value::Null;
    let mut model = Vec::new();
    if !remaining.is_empty() {
        match client.evaluate(state, Value::Object(questions)) {
            Ok(response) => {
                raw = response.clone();
                for (index, mention) in remaining.iter().enumerate() {
                    match choice(&response, &format!("m{index}")) {
                        Ok((selected, probabilities, confidence)) => {
                            let (decision, target) = decode(&selected, &prior_candidates[index]);
                            if decision == "invalid" {
                                error = json!(format!("invalid Jev choice: {selected}"));
                            }
                            model.push(json!({"candidate":mention,"current_mention":mention,
                                "mention_index":index,"decision":if decision=="invalid"{"unresolved"}else{decision},
                                "target_candidate":target,"prior_candidate_count":prior_candidates[index].len(),
                                "probabilities":probabilities,"confidence":confidence}));
                        }
                        Err(e) => {
                            error = json!(e);
                            model.push(fallback(index, mention, prior_candidates[index].len()));
                        }
                    }
                }
            }
            Err(e) => {
                error = json!(e);
                for (index, mention) in remaining.iter().enumerate() {
                    model.push(fallback(index, mention, prior_candidates[index].len()));
                }
            }
        }
    }

    let resolved: Vec<Value> = deterministic
        .iter()
        .map(|d| d["candidate"].clone())
        .chain(
            model
                .iter()
                .filter(|d| d["decision"] == "resolve_existing" || d["decision"] == "create_new")
                .map(|d| d["candidate"].clone()),
        )
        .collect();
    let preserved: Vec<Value> = resolved
        .iter()
        .cloned()
        .chain(
            model
                .iter()
                .filter(|d| d["decision"] == "unresolved")
                .map(|d| d["candidate"].clone()),
        )
        .collect();
    let rejected: Vec<Value> = model
        .iter()
        .filter(|d| d["decision"] == "reject")
        .map(|d| d["candidate"].clone())
        .collect();

    json!({"memory_id":id,"owner_id":owner,"expected_entity_mentions":row["expected_entity_mentions"],
        "baseline_entity_mentions":baseline,"deterministic_resolutions":deterministic,
        "model_decisions":model,"resolved_entity_mentions":resolved,"preserved_entity_mentions":preserved,
        "rejected_entity_mentions":rejected,"model_call":!remaining.is_empty(),"raw":raw,"error":error})
}

fn decode(selected: &str, candidates: &[Value]) -> (&'static str, Value) {
    if selected == "create_new" {
        return ("create_new", Value::Null);
    }
    if selected == "unresolved" {
        return ("unresolved", Value::Null);
    }
    if selected == "reject" {
        return ("reject", Value::Null);
    }
    let Some(index) = selected
        .strip_prefix("candidate_")
        .and_then(|v| v.parse::<usize>().ok())
    else {
        return ("invalid", Value::Null);
    };
    match candidates.get(index) {
        Some(candidate) => ("resolve_existing", candidate.clone()),
        None => ("invalid", Value::Null),
    }
}

fn fallback(index: usize, mention: &Value, count: usize) -> Value {
    json!({"candidate":mention,"current_mention":mention,"mention_index":index,
        "decision":"unresolved","target_candidate":null,"prior_candidate_count":count,
        "probabilities":null,"confidence":null})
}

fn truncate(value: &str, max: usize) -> &str {
    if value.len() <= max {
        return value;
    }
    let mut end = max;
    while !value.is_char_boundary(end) {
        end -= 1;
    }
    &value[..end]
}
