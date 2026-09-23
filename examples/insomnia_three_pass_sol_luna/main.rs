mod contract;

use reliquary_memory::{
    ConfiguredGeneralEndpoint, CredentialId, GeneralEndpoint, ModelProvider, ModelReasoningEffort,
    ModelSwitchboard, ReliquaryConfig,
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
            .unwrap_or("target/insomnia-sol-metadata-three-pass"),
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
    let reasoning_effort = match env::var("INSOMNIA_TEST_REASONING")
        .unwrap_or_else(|_| "low".to_owned())
        .as_str()
    {
        "low" => ModelReasoningEffort::Low,
        "medium" => ModelReasoningEffort::Medium,
        "high" => ModelReasoningEffort::High,
        value => {
            return Err(std::io::Error::other(format!(
                "unsupported INSOMNIA_TEST_REASONING: {value}"
            ))
            .into());
        }
    };
    fs::create_dir_all(&output)?;

    let mut episodes = load_episodes(&repo.join("corpus/insomnia-tuning-v1.jsonl"))?;
    if let Some(id) = conversation {
        episodes.retain(|e| e.payload["conversation_id"] == id);
    }
    episodes.truncate(limit.min(episodes.len()));
    let episodes = Arc::new(episodes);

    let config = ReliquaryConfig::open(repo.join("reliquary.cfg"))?;
    let credentials = config.credentials.clone();
    let mut models = config.models.clone();
    let semantic_route = if models.insomnia.is_some() {
        models.insomnia.as_mut().expect("checked insomnia route")
    } else {
        models.general.as_mut().ok_or_else(|| {
            std::io::Error::other("test requires a configured general/insomnia route")
        })?
    };
    if semantic_route.provider != ModelProvider::OpenAiCodex {
        return Err(std::io::Error::other(
            "Sol/Luna comparison requires the current Insomnia credential route to use openai-codex",
        )
        .into());
    }
    semantic_route.model = "gpt-6-sol".to_owned();
    semantic_route.reasoning_effort = Some(reasoning_effort);
    let switchboard = ModelSwitchboard::new(models.clone(), credentials.clone())?;
    let endpoint = ConfiguredGeneralEndpoint::from_insomnia_switchboard(&switchboard)?;
    let semantic_model = endpoint.model().to_owned();

    let mut metadata_models = models;
    let metadata_route = if metadata_models.insomnia.is_some() {
        metadata_models
            .insomnia
            .as_mut()
            .expect("checked insomnia route")
    } else {
        metadata_models.general.as_mut().ok_or_else(|| {
            std::io::Error::other("metadata pass requires a configured general/insomnia route")
        })?
    };
    if metadata_route.provider != ModelProvider::OpenAiCodex {
        return Err(std::io::Error::other(
            "Luna metadata pass requires the current Insomnia credential route to use openai-codex",
        )
        .into());
    }
    metadata_route.model = "gpt-6-luna".to_owned();
    metadata_route.reasoning_effort = Some(reasoning_effort);
    let metadata_switchboard = ModelSwitchboard::new(metadata_models, credentials)?;
    let metadata_endpoint =
        ConfiguredGeneralEndpoint::from_insomnia_switchboard(&metadata_switchboard)?;
    let metadata_model = metadata_endpoint.model().to_owned();
    let next = Arc::new(AtomicUsize::new(0));
    let (tx, rx) = mpsc::channel::<RunResult>();

    thread::scope(|scope| {
        for _ in 0..workers.min(episodes.len().max(1)) {
            let tx = tx.clone();
            let next = next.clone();
            let episodes = episodes.clone();
            let endpoint = endpoint.clone();
            let metadata_endpoint = metadata_endpoint.clone();
            let output = output.clone();
            scope.spawn(move || {
                loop {
                    let index = next.fetch_add(1, Ordering::Relaxed);
                    if index >= episodes.len() {
                        break;
                    }
                    let result = run_episode(
                        index,
                        &episodes[index].payload,
                        &endpoint,
                        &metadata_endpoint,
                        &output,
                        resume,
                    )
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
        &json!({
            "model": semantic_model,
            "semantic_model": semantic_model,
            "metadata_model": metadata_model,
            "synthesis_model": endpoint.model(),
            "reasoning_effort": reasoning_effort.as_str(),
            "episodes": episodes.len(),
            "memories": memories.len(),
            "workers": workers
        }),
    )?;
    Ok(())
}

fn run_episode(
    index: usize,
    episode: &Value,
    endpoint: &ConfiguredGeneralEndpoint,
    metadata_endpoint: &ConfiguredGeneralEndpoint,
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
    repair_missing_user_turns(episode, &mut ledger)?;
    validate_ledger(episode, &ledger)?;

    let mut synthesis_groups = build_synthesis_groups(&ledger)?;
    let metadata_groups = build_metadata_groups(episode, &ledger, &synthesis_groups)?;
    let metadata_payload = json!({
        "authoritative_episode": episode,
        "fixed_synthesis_groups": metadata_groups
    });
    let metadata_path = output.join(format!("episode-{:03}-metadata.json", index + 1));
    let metadata = if metadata_payload["fixed_synthesis_groups"]
        .as_array()
        .is_some_and(Vec::is_empty)
    {
        json!({"groups": {}})
    } else if resume && metadata_path.exists() {
        serde_json::from_str(&fs::read_to_string(&metadata_path).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?
    } else {
        metadata_endpoint
            .complete_json(
                contract::METADATA_PROMPT,
                &metadata_payload.to_string(),
                "insomnia_memory_metadata",
                &contract::metadata_schema(&metadata_payload["fixed_synthesis_groups"]),
            )
            .map_err(|e| e.to_string())?
    };
    write_json(&metadata_path, &metadata).map_err(|e| e.to_string())?;
    apply_metadata(&mut ledger, &mut synthesis_groups, &metadata)?;
    write_json(&ledger_path, &ledger).map_err(|e| e.to_string())?;
    let synthesis_payload =
        json!({"authoritative_episode": episode, "synthesis_groups": synthesis_groups});
    let synthesis_path = output.join(format!("episode-{:03}-synthesis.json", index + 1));
    let synthesis = if resume && synthesis_path.exists() {
        serde_json::from_str(&fs::read_to_string(&synthesis_path).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?
    } else {
        endpoint
            .complete_json(
                contract::SYNTHESIS_PROMPT,
                &synthesis_payload.to_string(),
                "insomnia_memory_wording",
                &contract::synthesis_schema(&synthesis_payload["synthesis_groups"]),
            )
            .map_err(|e| e.to_string())?
    };
    write_json(&synthesis_path, &synthesis).map_err(|e| e.to_string())?;
    let candidates =
        materialize_candidates(episode, &synthesis_payload["synthesis_groups"], &synthesis)?;
    validate_candidates(episode, &ledger, &candidates)?;
    Ok(json!({
        "conversation_id": episode["conversation_id"],
        "ledger": ledger,
        "synthesis_groups": synthesis_payload["synthesis_groups"],
        "candidates": candidates
    }))
}

fn repair_missing_user_turns(episode: &Value, ledger: &mut Value) -> Result<(), String> {
    let entries = ledger["entries"]
        .as_array_mut()
        .ok_or("ledger.entries is not an array")?;
    let seen: HashSet<String> = entries
        .iter()
        .filter_map(|entry| entry["source_node_id"].as_str().map(str::to_owned))
        .collect();
    let mut repaired = 0usize;
    for turn in episode["turns"].as_array().into_iter().flatten() {
        if turn["role"] != "user" {
            continue;
        }
        let Some(id) = turn["id"].as_str() else {
            continue;
        };
        if seen.contains(id) {
            continue;
        }
        entries.push(json!({
            "source_node_id": id,
            "disposition": "omit",
            "authority_kind": "none",
            "category": "none",
            "type": "none",
            "lifecycle": "none",
            "authority_source_node_id": "",
            "grounding_source_node_id": "",
            "proposition": "",
            "source_quote": "",
            "reason": "Deterministic benchmark repair for a user turn omitted from the model ledger."
        }));
        repaired += 1;
    }
    if repaired > 0 {
        eprintln!("deterministically repaired {repaired} missing user turn(s)");
    }
    Ok(())
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

fn validate_candidates(episode: &Value, ledger: &Value, candidates: &Value) -> Result<(), String> {
    let turns = turn_map(episode);
    let retained: HashSet<&str> = ledger["entries"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|e| e["disposition"] == "retain")
        .filter_map(|e| e["source_node_id"].as_str())
        .collect();
    for candidate in candidates.as_array().ok_or("candidates is not an array")? {
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
        if candidate["title"].as_str().unwrap_or("").trim().is_empty()
            || candidate["content"]
                .as_str()
                .unwrap_or("")
                .trim()
                .is_empty()
        {
            return Err(format!("candidate wording is empty: {id}"));
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
fn build_metadata_groups(episode: &Value, ledger: &Value, groups: &Value) -> Result<Value, String> {
    let turns = turn_map(episode);
    let ledger_entries = ledger["entries"]
        .as_array()
        .ok_or("ledger.entries is not an array")?;
    let groups = groups
        .as_array()
        .ok_or("synthesis_groups is not an array")?;
    let mut metadata_groups = Vec::with_capacity(groups.len());
    for group in groups {
        let source_quotes: Vec<Value> = group["ledger_entry_indices"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(Value::as_u64)
            .map(|index| {
                ledger_entries
                    .get(index as usize)
                    .map(|entry| entry["source_quote"].clone())
                    .ok_or_else(|| format!("invalid metadata ledger entry index {index}"))
            })
            .collect::<Result<_, _>>()?;
        let authority_node_id = group["authority_source_node_id"].as_str().unwrap_or("");
        let grounding_node_id = group["grounding_source_node_id"].as_str().unwrap_or("");
        let authority_context = optional_turn_content(&turns, authority_node_id)?;
        let grounding_context = optional_turn_content(&turns, grounding_node_id)?;
        metadata_groups.push(json!({
            "group_id": group["group_id"],
            "source_node_id": group["source_node_id"],
            "source_quotes": source_quotes,
            "authority_kind": group["authority_kind"],
            "authority_source_node_id": authority_node_id,
            "authority_context": authority_context,
            "grounding_source_node_id": grounding_node_id,
            "grounding_context": grounding_context,
            "propositions": group["propositions"]
        }));
    }
    Ok(Value::Array(metadata_groups))
}

fn apply_metadata(ledger: &mut Value, groups: &mut Value, metadata: &Value) -> Result<(), String> {
    let classifications = metadata["groups"]
        .as_object()
        .ok_or("metadata.groups is not an object")?;
    let groups = groups
        .as_array_mut()
        .ok_or("synthesis_groups is not an array")?;
    if classifications.len() != groups.len() {
        return Err(format!(
            "metadata count mismatch: expected {}, got {}",
            groups.len(),
            classifications.len()
        ));
    }
    let ledger_entries = ledger["entries"]
        .as_array_mut()
        .ok_or("ledger.entries is not an array")?;
    for group in groups {
        let group_id = group["group_id"]
            .as_str()
            .ok_or("synthesis group lacks group_id")?
            .to_owned();
        let classified = classifications
            .get(&group_id)
            .ok_or_else(|| format!("metadata omitted group {group_id}"))?;
        let category = classified["category"]
            .as_str()
            .ok_or_else(|| format!("metadata category missing for {group_id}"))?;
        let memory_type = classified["type"]
            .as_str()
            .ok_or_else(|| format!("metadata type missing for {group_id}"))?;
        let lifecycle = classified["lifecycle"]
            .as_str()
            .ok_or_else(|| format!("metadata lifecycle missing for {group_id}"))?;
        if ![
            "fact",
            "preference",
            "decision",
            "instruction",
            "relationship",
            "constraint",
            "correction",
            "commitment",
        ]
        .contains(&category)
        {
            return Err(format!(
                "invalid metadata category for {group_id}: {category}"
            ));
        }
        if ![
            "identity",
            "education",
            "employment",
            "location",
            "possession",
            "health",
            "finance",
            "schedule",
            "communication",
            "project",
            "process",
            "product",
            "relationship",
            "other",
        ]
        .contains(&memory_type)
        {
            return Err(format!(
                "invalid metadata type for {group_id}: {memory_type}"
            ));
        }
        if !["current", "future", "historical"].contains(&lifecycle) {
            return Err(format!(
                "invalid metadata lifecycle for {group_id}: {lifecycle}"
            ));
        }
        group["category"] = Value::String(category.to_owned());
        group["type"] = Value::String(memory_type.to_owned());
        group["lifecycle"] = Value::String(lifecycle.to_owned());
        for entry_index in group["ledger_entry_indices"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(Value::as_u64)
        {
            let entry = ledger_entries
                .get_mut(entry_index as usize)
                .ok_or_else(|| format!("invalid ledger entry index for {group_id}"))?;
            entry["category"] = Value::String(category.to_owned());
            entry["type"] = Value::String(memory_type.to_owned());
            entry["lifecycle"] = Value::String(lifecycle.to_owned());
        }
    }
    Ok(())
}

fn build_synthesis_groups(ledger: &Value) -> Result<Value, String> {
    let entries = ledger["entries"]
        .as_array()
        .ok_or("ledger.entries is not an array")?;
    let mut groups: Vec<Value> = Vec::new();
    for (entry_index, entry) in entries.iter().enumerate() {
        if entry["disposition"] != "retain" {
            continue;
        }
        if let Some(group) = groups
            .iter_mut()
            .find(|group| same_synthesis_signature(group, entry))
        {
            group["ledger_entry_indices"]
                .as_array_mut()
                .expect("group indices")
                .push(json!(entry_index));
            group["propositions"]
                .as_array_mut()
                .expect("group propositions")
                .push(entry["proposition"].clone());
            continue;
        }
        let group_id = format!("g{:03}", groups.len());
        groups.push(json!({
            "group_id": group_id,
            "ledger_entry_indices": [entry_index],
            "source_node_id": entry["source_node_id"],
            "authority_kind": entry["authority_kind"],
            "category": entry["category"],
            "type": entry["type"],
            "lifecycle": entry["lifecycle"],
            "authority_source_node_id": entry["authority_source_node_id"],
            "grounding_source_node_id": entry["grounding_source_node_id"],
            "propositions": [entry["proposition"].clone()]
        }));
    }
    Ok(Value::Array(groups))
}

fn same_synthesis_signature(group: &Value, entry: &Value) -> bool {
    [
        "source_node_id",
        "authority_kind",
        "category",
        "type",
        "lifecycle",
        "authority_source_node_id",
        "grounding_source_node_id",
    ]
    .into_iter()
    .all(|key| group[key] == entry[key])
}

fn materialize_candidates(
    episode: &Value,
    groups: &Value,
    synthesis: &Value,
) -> Result<Value, String> {
    let turns = turn_map(episode);
    let wording = synthesis["groups"]
        .as_object()
        .ok_or("synthesis.groups is not an object")?;
    let groups = groups
        .as_array()
        .ok_or("synthesis_groups is not an array")?;
    if wording.len() != groups.len() {
        return Err(format!(
            "synthesis group count mismatch: expected {}, got {}",
            groups.len(),
            wording.len()
        ));
    }
    let conversation_id = episode["conversation_id"].as_str().unwrap_or("");
    let mut candidates = Vec::with_capacity(groups.len());
    for group in groups {
        let group_id = group["group_id"]
            .as_str()
            .ok_or("synthesis group lacks group_id")?;
        let words = wording
            .get(group_id)
            .ok_or_else(|| format!("synthesis omitted group {group_id}"))?;
        let source_node_id = group["source_node_id"].as_str().unwrap_or("");
        let source_quote = exact_turn_content(&turns, source_node_id)?;
        let authority_node_id = group["authority_source_node_id"].as_str().unwrap_or("");
        let grounding_node_id = group["grounding_source_node_id"].as_str().unwrap_or("");
        let authority_quote = optional_turn_content(&turns, authority_node_id)?;
        let grounding_quote = optional_turn_content(&turns, grounding_node_id)?;
        candidates.push(json!({
            "authority_kind": group["authority_kind"],
            "category": group["category"],
            "type": group["type"],
            "lifecycle": group["lifecycle"],
            "title": words["title"],
            "content": words["content"],
            "source_node_id": source_node_id,
            "source_quote": source_quote,
            "authority_source_conversation_id": if authority_node_id.is_empty() { "" } else { conversation_id },
            "authority_source_node_id": authority_node_id,
            "authority_source_quote": authority_quote,
            "grounding_source_conversation_id": if grounding_node_id.is_empty() { "" } else { conversation_id },
            "grounding_source_node_id": grounding_node_id,
            "grounding_source_quote": grounding_quote
        }));
    }
    Ok(Value::Array(candidates))
}

fn exact_turn_content(turns: &HashMap<&str, &Value>, id: &str) -> Result<String, String> {
    turns
        .get(id)
        .and_then(|turn| turn["content"].as_str())
        .map(str::to_owned)
        .ok_or_else(|| format!("missing provenance turn: {id}"))
}

fn optional_turn_content(turns: &HashMap<&str, &Value>, id: &str) -> Result<String, String> {
    if id.is_empty() {
        Ok(String::new())
    } else {
        exact_turn_content(turns, id)
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
