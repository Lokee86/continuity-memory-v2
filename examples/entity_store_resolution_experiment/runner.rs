use reliquary_memory::{
    ConfiguredGeneralEndpoint, ENTITY_RESOLVER_SYSTEM_PROMPT, GeneralEndpoint,
    entity_resolver_schema,
};
use serde_json::{Value, json};
use std::{
    collections::{HashMap, HashSet},
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

type Store = HashMap<(String, String), Vec<Value>>;

pub fn run(
    endpoint: ConfiguredGeneralEndpoint,
    queries: Vec<Value>,
    entities: Vec<Value>,
    workers: usize,
) -> Result<Vec<Value>, String> {
    let store = Arc::new(build_store(entities)?);
    let endpoint = Arc::new(endpoint);
    let queries = Arc::new(queries);
    let next = Arc::new(Mutex::new(0usize));
    let (tx, rx) = mpsc::channel();

    thread::scope(|scope| {
        for _ in 0..workers.min(queries.len().max(1)) {
            let (endpoint, queries, store, next, tx) = (
                endpoint.clone(),
                queries.clone(),
                store.clone(),
                next.clone(),
                tx.clone(),
            );
            scope.spawn(move || {
                loop {
                    let index = {
                        let mut next = next.lock().unwrap();
                        if *next >= queries.len() {
                            break;
                        }
                        let index = *next;
                        *next += 1;
                        index
                    };
                    if tx
                        .send((index, run_query(&endpoint, &queries[index], &store)))
                        .is_err()
                    {
                        break;
                    }
                }
            });
        }
        drop(tx);
    });

    let mut rows = vec![None; queries.len()];
    for (index, row) in rx {
        rows[index] = Some(row?);
    }
    rows.into_iter()
        .map(|row| row.ok_or_else(|| "worker omitted query".to_owned()))
        .collect()
}

fn build_store(entities: Vec<Value>) -> Result<Store, String> {
    let mut store: Store = HashMap::new();
    for entity in entities {
        let owner = entity["owner_id"]
            .as_str()
            .ok_or("entity owner_id missing")?;
        let entity_id = entity["entity_id"].as_str().ok_or("entity_id missing")?;
        let mut surfaces = HashSet::new();
        if let Some(name) = entity["canonical_name"].as_str() {
            surfaces.insert(norm(name));
        }
        for alias in entity["aliases"].as_array().into_iter().flatten() {
            if let Some(alias) = alias.as_str() {
                surfaces.insert(norm(alias));
            }
        }
        for surface in surfaces {
            let bucket = store.entry((owner.to_owned(), surface)).or_default();
            if !bucket.iter().any(|value| value["entity_id"] == entity_id) {
                bucket.push(entity.clone());
            }
        }
    }
    for bucket in store.values_mut() {
        bucket.sort_by(|a, b| a["entity_id"].as_str().cmp(&b["entity_id"].as_str()));
        bucket.truncate(8);
    }
    Ok(store)
}

fn run_query(
    endpoint: &ConfiguredGeneralEndpoint,
    query: &Value,
    store: &Store,
) -> Result<Value, String> {
    let memory_id = query["memory_id"].as_str().ok_or("memory_id missing")?;
    let owner = query["owner_id"].as_str().ok_or("owner_id missing")?;
    let mention = query["mentions"]
        .as_array()
        .ok_or("mentions missing")?
        .first()
        .ok_or("mention missing")?;
    let surface = mention["text"].as_str().ok_or("mention text missing")?;
    let candidates = store
        .get(&(owner.to_owned(), norm(surface)))
        .cloned()
        .unwrap_or_default();

    let mut results = Vec::new();
    for order in candidate_orders(&candidates) {
        results.push(run_order(endpoint, query, mention, &order)?);
    }

    let order_invariant = results.iter().skip(1).all(|result| {
        result["decision"] == results[0]["decision"]
            && result["target_entity_id"] == results[0]["target_entity_id"]
    });
    let consensus = if order_invariant {
        json!({
            "decision": results[0]["decision"],
            "target_entity_id": results[0]["target_entity_id"],
        })
    } else {
        Value::Null
    };

    Ok(json!({
        "memory_id": memory_id,
        "owner_id": owner,
        "mention": mention,
        "base_candidate_entity_ids": candidates.iter()
            .map(|entity| entity["entity_id"].clone())
            .collect::<Vec<_>>(),
        "permutation_results": results,
        "consensus": consensus,
        "order_invariant": order_invariant,
    }))
}

fn run_order(
    endpoint: &ConfiguredGeneralEndpoint,
    query: &Value,
    mention: &Value,
    order: &[Value],
) -> Result<Value, String> {
    let payload = json!({
        "title": query["title"],
        "content": query["content"],
        "mention": mention,
        "candidates": order.iter().enumerate().map(|(index, entity)| json!({
            "candidate_index": index,
            "entity_id": entity["entity_id"],
            "canonical_name": entity["canonical_name"],
            "aliases": entity["aliases"],
            "kind": entity["kind"],
            "summary": entity["summary"],
            "evidence": entity["evidence"],
        })).collect::<Vec<_>>(),
    });
    let raw = endpoint
        .complete_json(
            ENTITY_RESOLVER_SYSTEM_PROMPT,
            &payload.to_string(),
            "entity_store_resolution_experiment_v1",
            &entity_resolver_schema(),
        )
        .map_err(|error| error.to_string())?;

    match validate(&raw, order.len()) {
        Ok((decision, reason, target_index)) => {
            let target_entity_id = if decision == "resolve_existing" {
                Some(order[target_index.unwrap()]["entity_id"].clone())
            } else {
                None
            };
            Ok(json!({
                "entity_id_order": order.iter().map(|entity| entity["entity_id"].clone()).collect::<Vec<_>>(),
                "decision": decision,
                "reason": reason,
                "target_candidate_index": target_index.map(|value| value as i64).unwrap_or(-1),
                "target_entity_id": target_entity_id,
                "raw": raw,
                "error": Value::Null,
            }))
        }
        Err(error) => Ok(json!({
            "entity_id_order": order.iter().map(|entity| entity["entity_id"].clone()).collect::<Vec<_>>(),
            "decision": "unresolved",
            "reason": "insufficient_evidence",
            "target_candidate_index": -1,
            "target_entity_id": Value::Null,
            "raw": raw,
            "error": error,
        })),
    }
}

fn validate(
    raw: &Value,
    candidate_count: usize,
) -> Result<(String, String, Option<usize>), String> {
    let decision = raw["decision"].as_str().ok_or("bad decision")?;
    if !["resolve_existing", "create_new", "unresolved", "reject"].contains(&decision) {
        return Err("bad decision".into());
    }
    let reason = raw["reason"].as_str().ok_or("bad reason")?;
    if !REASONS.contains(&reason) {
        return Err("bad reason".into());
    }
    if decision == "resolve_existing" {
        let target = raw["target_candidate_index"].as_i64().ok_or("bad target")?;
        let target = usize::try_from(target).map_err(|_| "target out of range")?;
        if target >= candidate_count {
            return Err("target out of range".into());
        }
        Ok((decision.to_owned(), reason.to_owned(), Some(target)))
    } else {
        Ok((decision.to_owned(), reason.to_owned(), None))
    }
}

fn candidate_orders(candidates: &[Value]) -> Vec<Vec<Value>> {
    if candidates.len() <= 1 {
        return vec![candidates.to_vec()];
    }
    if candidates.len() > 4 {
        return vec![candidates.to_vec()];
    }
    let mut values = candidates.to_vec();
    let mut out = Vec::new();
    permute(&mut values, 0, &mut out);
    out
}

fn permute(values: &mut [Value], start: usize, out: &mut Vec<Vec<Value>>) {
    if start == values.len() {
        out.push(values.to_vec());
        return;
    }
    for index in start..values.len() {
        values.swap(start, index);
        permute(values, start + 1, out);
        values.swap(start, index);
    }
}

fn norm(value: &str) -> String {
    value.trim().to_ascii_lowercase()
}
