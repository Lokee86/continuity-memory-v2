mod contract;

use continuity_memory::{
    ConfiguredGeneralEndpoint, ContinuityConfig, GeneralEndpoint, ModelReasoningEffort,
    ModelSwitchboard,
};
use serde_json::{Map, Value, json};
use std::collections::{HashMap, HashSet};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
    mpsc,
};
use std::thread;

#[derive(Clone)]
struct Episode {
    payload: Value,
    first_timestamp: i64,
}

type RunResult = Result<(usize, Value), String>;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let args: Vec<String> = env::args().skip(1).collect();
    let output = repo.join(
        args.first()
            .map(String::as_str)
            .unwrap_or("target/insomnia-sol-two-pass"),
    );
    let workers = args
        .get(1)
        .and_then(|v| v.parse().ok())
        .unwrap_or(4usize)
        .clamp(1, 16);
    let limit = args
        .get(2)
        .and_then(|v| v.parse().ok())
        .unwrap_or(usize::MAX);
    let resume = args.iter().any(|arg| arg == "--resume");
    let conversation = args
        .get(3)
        .filter(|arg| !arg.starts_with("--"))
        .map(String::as_str);
    fs::create_dir_all(&output)?;

    let mut episodes = load_episodes(&repo.join("corpus/insomnia-tuning-v1.jsonl"))?;
    if let Some(id) = conversation {
        episodes.retain(|e| e.payload["conversation_id"] == id);
    }
    episodes.truncate(limit.min(episodes.len()));
    let episodes = Arc::new(episodes);

    let config = ContinuityConfig::open(repo.join("continuity.cfg"))?;
    let mut models = config.models;
    if let Some(route) = models.insomnia.as_mut() {
        route.reasoning_effort = Some(ModelReasoningEffort::Low);
    } else if let Some(route) = models.general.as_mut() {
        route.reasoning_effort = Some(ModelReasoningEffort::Low);
    }
    let switchboard = ModelSwitchboard::new(models, config.credentials)?;
    let endpoint = ConfiguredGeneralEndpoint::from_insomnia_switchboard(&switchboard)?;
    let model = endpoint.model().to_owned();
    let next = Arc::new(AtomicUsize::new(0));
    let (tx, rx) = mpsc::channel::<RunResult>();

    thread::scope(|scope| {
        for _ in 0..workers.min(episodes.len().max(1)) {
            let tx = tx.clone();
            let next = next.clone();
            let episodes = episodes.clone();
            let endpoint = endpoint.clone();
            let output = output.clone();
            scope.spawn(move || {
                loop {
                    let index = next.fetch_add(1, Ordering::Relaxed);
                    if index >= episodes.len() {
                        break;
                    }
                    let result =
                        run_episode(index, &episodes[index].payload, &endpoint, &output, resume)
                            .map(|value| (index, value));
                    if tx.send(result).is_err() {
                        break;
                    }
                }
            });
        }
        drop(tx);
    });

    let mut ordered = vec![None; episodes.len()];
    for result in rx {
        let (index, value) = result.map_err(std::io::Error::other)?;
        println!(
            "episode {}/{}: {} candidates",
            index + 1,
            episodes.len(),
            value["candidates"].as_array().map_or(0, Vec::len)
        );
        ordered[index] = Some(value);
    }
    let results: Vec<Value> = ordered
        .into_iter()
        .map(|v| v.expect("worker result"))
        .collect();
    let memories: Vec<Value> = results
        .iter()
        .flat_map(|r| {
            r["candidates"]
                .as_array()
                .into_iter()
                .flatten()
                .map(memory_row)
        })
        .collect();
    write_json(&output.join("run.json"), &Value::Array(results))?;
    write_json(
        &output.join("memories.json"),
        &Value::Array(memories.clone()),
    )?;
    write_json(
        &output.join("meta.json"),
        &json!({"model": model, "reasoning_effort": "low", "episodes": episodes.len(), "memories": memories.len(), "workers": workers}),
    )?;
    Ok(())
}

fn run_episode(
    index: usize,
    episode: &Value,
    endpoint: &ConfiguredGeneralEndpoint,
    output: &Path,
    resume: bool,
) -> Result<Value, String> {
    let ledger_path = output.join(format!("episode-{:03}-ledger.json", index + 1));
    let mut ledger = if resume && ledger_path.exists() {
        serde_json::from_str(&fs::read_to_string(&ledger_path).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?
    } else {
        endpoint
            .complete_json(
                contract::LEDGER_PROMPT,
                &episode.to_string(),
                "insomnia_authority_ledger",
                &contract::ledger_schema(episode),
            )
            .map_err(|e| e.to_string())?
    };
    canonicalize_ledger_quotes(episode, &mut ledger);
    write_json(&ledger_path, &ledger).map_err(|e| e.to_string())?;
    validate_ledger(episode, &ledger)?;
    let synthesis_payload =
        json!({"authoritative_episode": episode, "authority_disposition_ledger": ledger});
    let synthesis_path = output.join(format!("episode-{:03}-synthesis.json", index + 1));
    let mut synthesis = if resume && synthesis_path.exists() {
        serde_json::from_str(&fs::read_to_string(&synthesis_path).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?
    } else {
        endpoint
            .complete_json(
                contract::SYNTHESIS_PROMPT,
                &synthesis_payload.to_string(),
                "insomnia_memory_synthesis",
                &contract::synthesis_schema(
                    episode,
                    &synthesis_payload["authority_disposition_ledger"],
                ),
            )
            .map_err(|e| e.to_string())?
    };
    canonicalize_synthesis_provenance(episode, &mut synthesis);
    write_json(&synthesis_path, &synthesis).map_err(|e| e.to_string())?;
    validate_candidates(
        episode,
        &synthesis_payload["authority_disposition_ledger"],
        &synthesis,
    )?;
    Ok(
        json!({"conversation_id": episode["conversation_id"], "ledger": synthesis_payload["authority_disposition_ledger"], "candidates": synthesis["candidates"]}),
    )
}

fn validate_ledger(episode: &Value, ledger: &Value) -> Result<(), String> {
    let turns = turn_map(episode);
    let users: HashSet<&str> = turns
        .iter()
        .filter(|(_, t)| t["role"] == "user")
        .map(|(id, _)| *id)
        .collect();
    let entries = ledger["entries"]
        .as_array()
        .ok_or("ledger.entries is not an array")?;
    let mut seen = HashSet::new();
    for entry in entries {
        let id = entry["source_node_id"]
            .as_str()
            .ok_or("ledger source_node_id is not a string")?;
        if !users.contains(id) {
            return Err(format!("ledger source is not a user turn: {id}"));
        }
        seen.insert(id);
        let disposition = entry["disposition"].as_str().unwrap_or("");
        if disposition != "omit" {
            let quote = entry["source_quote"].as_str().unwrap_or("");
            if quote.is_empty() || !turns[id]["content"].as_str().unwrap_or("").contains(quote) {
                return Err(format!("non-exact source quote: {id}"));
            }
        }
        if disposition == "retain"
            && entry["authority_kind"] == "adoption"
            && entry["authority_source_node_id"]
                .as_str()
                .unwrap_or("")
                .is_empty()
        {
            return Err(format!("adoption lacks authority source: {id}"));
        }
    }
    let missing: Vec<_> = users.difference(&seen).copied().collect();
    if !missing.is_empty() {
        return Err(format!("ledger omitted {} user turns", missing.len()));
    }
    Ok(())
}

fn validate_candidates(episode: &Value, ledger: &Value, synthesis: &Value) -> Result<(), String> {
    let turns = turn_map(episode);
    let retained: HashSet<&str> = ledger["entries"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|e| e["disposition"] == "retain")
        .filter_map(|e| e["source_node_id"].as_str())
        .collect();
    for candidate in synthesis["candidates"]
        .as_array()
        .ok_or("candidates is not an array")?
    {
        let id = candidate["source_node_id"]
            .as_str()
            .ok_or("candidate source_node_id is not a string")?;
        if !retained.contains(id) {
            return Err(format!("candidate bypassed ledger: {id}"));
        }
        let quote = candidate["source_quote"].as_str().unwrap_or("");
        if quote.is_empty() || !turns[id]["content"].as_str().unwrap_or("").contains(quote) {
            return Err(format!("candidate quote is not exact: {id}"));
        }
    }
    Ok(())
}

fn load_episodes(path: &Path) -> Result<Vec<Episode>, Box<dyn std::error::Error>> {
    let mut grouped: HashMap<String, Vec<(i64, usize, Value)>> = HashMap::new();
    for (order, line) in fs::read_to_string(path)?.lines().enumerate() {
        let row: Value = serde_json::from_str(line)?;
        if row["kind"] != "node" {
            continue;
        }
        let id = row["conversation_id"]
            .as_str()
            .ok_or("missing conversation_id")?
            .to_owned();
        grouped.entry(id).or_default().push((
            row["timestamp_ns"].as_i64().unwrap_or(0),
            order,
            row,
        ));
    }
    let mut episodes = Vec::new();
    for (conversation_id, mut rows) in grouped {
        rows.sort_by_key(|(timestamp, order, _)| (*timestamp, *order));
        let first_timestamp = rows.first().map_or(0, |v| v.0);
        let turns: Vec<Value> = rows.into_iter().map(|(_, _, row)| json!({"id": row["id"], "role": row["role"], "timestamp_ns": row["timestamp_ns"], "content": row["content"]})).collect();
        episodes.push(Episode {
            payload: json!({"conversation_id": conversation_id, "turns": turns}),
            first_timestamp,
        });
    }
    episodes.sort_by_key(|episode| episode.first_timestamp);
    Ok(episodes)
}

fn turn_map(episode: &Value) -> HashMap<&str, &Value> {
    episode["turns"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|turn| turn["id"].as_str().map(|id| (id, turn)))
        .collect()
}
fn canonicalize_ledger_quotes(episode: &Value, ledger: &mut Value) {
    let turns = turn_map(episode);
    let Some(entries) = ledger["entries"].as_array_mut() else {
        return;
    };
    for entry in entries {
        if entry["disposition"] == "omit" {
            continue;
        }
        let Some(id) = entry["source_node_id"].as_str() else {
            continue;
        };
        let Some(source) = turns.get(id).and_then(|turn| turn["content"].as_str()) else {
            continue;
        };
        let exact = entry["source_quote"]
            .as_str()
            .is_some_and(|quote| !quote.is_empty() && source.contains(quote));
        if !exact {
            entry["source_quote"] = Value::String(source.to_owned());
        }
    }
}
fn canonicalize_synthesis_provenance(episode: &Value, synthesis: &mut Value) {
    let turns = turn_map(episode);
    let conversation = episode["conversation_id"].as_str().unwrap_or("").to_owned();
    let Some(candidates) = synthesis["candidates"].as_array_mut() else {
        return;
    };
    for candidate in candidates {
        canonicalize_candidate_source(&turns, candidate, "source_node_id", "source_quote");
        canonicalize_candidate_source(
            &turns,
            candidate,
            "authority_source_node_id",
            "authority_source_quote",
        );
        canonicalize_candidate_source(
            &turns,
            candidate,
            "grounding_source_node_id",
            "grounding_source_quote",
        );
        if !candidate["authority_source_node_id"]
            .as_str()
            .unwrap_or("")
            .is_empty()
        {
            candidate["authority_source_conversation_id"] = Value::String(conversation.clone());
        }
        if !candidate["grounding_source_node_id"]
            .as_str()
            .unwrap_or("")
            .is_empty()
        {
            candidate["grounding_source_conversation_id"] = Value::String(conversation.clone());
        }
    }
}
fn canonicalize_candidate_source(
    turns: &HashMap<&str, &Value>,
    candidate: &mut Value,
    id_key: &str,
    quote_key: &str,
) {
    let Some(id) = candidate[id_key].as_str() else {
        return;
    };
    if id.is_empty() {
        candidate[quote_key] = Value::String(String::new());
        return;
    }
    let Some(source) = turns.get(id).and_then(|turn| turn["content"].as_str()) else {
        return;
    };
    let exact = candidate[quote_key]
        .as_str()
        .is_some_and(|quote| !quote.is_empty() && source.contains(quote));
    if !exact {
        candidate[quote_key] = Value::String(source.to_owned());
    }
}
fn memory_row(candidate: &Value) -> Value {
    let mut object = candidate.as_object().cloned().unwrap_or_else(Map::new);
    object.insert(
        "content_source_node_id".into(),
        candidate["authority_source_node_id"].clone(),
    );
    Value::Object(object)
}
fn write_json(path: &Path, value: &Value) -> std::io::Result<()> {
    fs::write(
        path,
        format!(
            "{}\n",
            serde_json::to_string_pretty(value).expect("serializable")
        ),
    )
}
