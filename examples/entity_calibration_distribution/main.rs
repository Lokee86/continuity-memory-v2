use reliquary_memory::{
    ConfiguredGeneralEndpoint, GeneralEndpoint, InsomniaCandidate, InsomniaExtractionError,
    InsomniaOwnership, MemoryTextField, ModelProvider, ModelReasoningEffort, ModelSwitchboard,
    ReliquaryConfig, enrich_entity_calibration,
};
use serde_json::{Value, json};
use std::collections::HashMap;
use std::error::Error;
use std::fs::{self, File};
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, mpsc};
use std::thread;
use std::time::Instant;

fn main() -> Result<(), Box<dyn Error>> {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    if args.len() < 3 {
        return Err("usage: entity_calibration_distribution <population.jsonl> <candidates.jsonl> <output-dir> [workers] [low|medium|high] [batch-size] [model]".into());
    }
    let workers = parse_usize(args.get(3), 8, 1, 16);
    let effort = parse_effort(args.get(4).map(String::as_str).unwrap_or("high"))?;
    let batch_size = parse_usize(args.get(5), 8, 1, 64);
    let model = args.get(6).map(String::as_str).unwrap_or("gpt-5.6-luna");
    let population = load_jsonl(&args[0])?;
    let sources = load_jsonl(&args[1])?
        .into_iter()
        .filter_map(|row| Some((row["memory_id"].as_str()?.to_owned(), row)))
        .collect::<HashMap<_, _>>();
    let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let (endpoint, model) = endpoint(&repo, effort, model)?;
    let started = Instant::now();
    let rows = run(endpoint, population, sources, workers, batch_size)?;
    let output = PathBuf::from(&args[2]);
    fs::create_dir_all(&output)?;
    write_jsonl(&output.join("results.jsonl"), &rows)?;
    let summary = summarize(
        &rows,
        &model,
        effort.as_str(),
        batch_size,
        started.elapsed().as_millis(),
    );
    fs::write(
        output.join("summary.json"),
        serde_json::to_string_pretty(&summary)?,
    )?;
    println!("{}", serde_json::to_string_pretty(&summary)?);
    Ok(())
}

fn run(
    endpoint: ConfiguredGeneralEndpoint,
    population: Vec<Value>,
    sources: HashMap<String, Value>,
    workers: usize,
    batch_size: usize,
) -> Result<Vec<Value>, String> {
    let endpoint = Arc::new(endpoint);
    let population = Arc::new(population);
    let sources = Arc::new(sources);
    let batches = population.len().div_ceil(batch_size);
    let next = Arc::new(Mutex::new(0usize));
    let (tx, rx) = mpsc::channel();
    thread::scope(|scope| {
        for _ in 0..workers.min(batches.max(1)) {
            let (endpoint, population, sources, next, tx) = (
                Arc::clone(&endpoint),
                Arc::clone(&population),
                Arc::clone(&sources),
                Arc::clone(&next),
                tx.clone(),
            );
            scope.spawn(move || {
                loop {
                    let batch = {
                        let mut n = next.lock().unwrap();
                        if *n >= batches {
                            break;
                        }
                        let v = *n;
                        *n += 1;
                        v
                    };
                    let start = batch * batch_size;
                    let end = (start + batch_size).min(population.len());
                    if tx
                        .send((
                            batch,
                            run_batch(&endpoint, &population[start..end], &sources, start),
                        ))
                        .is_err()
                    {
                        break;
                    }
                }
            });
        }
        drop(tx);
    });
    let mut output = vec![None; population.len()];
    let mut done = 0usize;
    for (batch, result) in rx {
        let start = batch * batch_size;
        let rows = result?;
        for (offset, row) in rows.into_iter().enumerate() {
            output[start + offset] = Some(row);
        }
        done += 1;
        if done % 16 == 0 || done == batches {
            println!("completed {done}/{batches} batches");
        }
    }
    output
        .into_iter()
        .map(|row| row.ok_or_else(|| "distribution worker omitted a case".into()))
        .collect()
}

fn run_batch(
    endpoint: &ConfiguredGeneralEndpoint,
    population: &[Value],
    sources: &HashMap<String, Value>,
    start: usize,
) -> Result<Vec<Value>, String> {
    let mut candidates = population
        .iter()
        .enumerate()
        .map(|(offset, row)| prepare(row, sources, start + offset))
        .collect::<Result<Vec<_>, _>>()?;
    match enrich_entity_calibration(endpoint, &mut candidates) {
        Ok(()) => Ok(population
            .iter()
            .zip(candidates.iter())
            .map(|(row, candidate)| result_row(row, candidate, None, false))
            .collect()),
        Err(InsomniaExtractionError::Endpoint(error)) => Err(error.to_string()),
        Err(InsomniaExtractionError::InvalidOutput(batch_error)) => {
            let mut rows = Vec::with_capacity(candidates.len());
            for (offset, row) in population.iter().enumerate() {
                let mut candidate = prepare(row, sources, start + offset)?;
                let error =
                    match enrich_entity_calibration(endpoint, std::slice::from_mut(&mut candidate))
                    {
                        Ok(()) => None,
                        Err(InsomniaExtractionError::InvalidOutput(error)) => Some(error),
                        Err(InsomniaExtractionError::Endpoint(error)) => {
                            return Err(error.to_string());
                        }
                    };
                rows.push(result_row(
                    row,
                    &candidate,
                    error.as_deref().or(Some(&batch_error)),
                    true,
                ));
            }
            Ok(rows)
        }
    }
}

fn prepare(
    row: &Value,
    sources: &HashMap<String, Value>,
    index: usize,
) -> Result<InsomniaCandidate, String> {
    let memory_id = row["memory_id"]
        .as_str()
        .ok_or("population memory_id missing")?;
    let source = sources
        .get(memory_id)
        .ok_or_else(|| format!("Memory missing from candidates: {memory_id}"))?;
    Ok(InsomniaCandidate {
        key: format!("case-{index:05}"),
        authority_kind: "direct".into(),
        category: "fact".into(),
        memory_type: "project".into(),
        temporal_status: "current".into(),
        ownership: if row["owner_kind"] == "phy" {
            InsomniaOwnership::User
        } else {
            InsomniaOwnership::Project
        },
        title: source["title"]
            .as_str()
            .ok_or("candidate title missing")?
            .into(),
        content: source["content"]
            .as_str()
            .ok_or("candidate content missing")?
            .into(),
        source_node_id: "calibration".into(),
        source_quote: "calibration".into(),
        authority_source_conversation_id: None,
        authority_source_node_id: None,
        authority_source_quote: None,
        grounding_source_conversation_id: None,
        grounding_source_node_id: None,
        grounding_source_quote: None,
        routing_metadata: None,
    })
}

fn result_row(
    source: &Value,
    candidate: &InsomniaCandidate,
    error: Option<&str>,
    batch_fallback: bool,
) -> Value {
    let routing = candidate.routing_metadata.as_ref();
    let mentions = routing.map(|r| r.entity_mentions.iter().map(|m| json!({"field": match m.field { MemoryTextField::Title => "title", MemoryTextField::Content => "content" }, "start_byte":m.start_byte,"end_byte":m.end_byte,"text":m.text})).collect::<Vec<_>>()).unwrap_or_default();
    json!({"owner_kind":source["owner_kind"],"memory_id":source["memory_id"],"body_id":source["body_id"],"actual_entity_mentions":mentions,"batch_fallback":batch_fallback,"error":error})
}

fn summarize(
    rows: &[Value],
    model: &str,
    effort: &str,
    batch_size: usize,
    elapsed_ms: u128,
) -> Value {
    let mentions = rows
        .iter()
        .map(|r| r["actual_entity_mentions"].as_array().map_or(0, Vec::len))
        .collect::<Vec<_>>();
    let owner = |kind: &str| rows.iter().filter(|r| r["owner_kind"] == kind).count();
    json!({"model":model,"reasoning_effort":effort,"cases":rows.len(),"batch_size":batch_size,"elapsed_ms":elapsed_ms,"errors":rows.iter().filter(|r| !r["error"].is_null()).count(),"batch_fallback_cases":rows.iter().filter(|r| r["batch_fallback"]==true).count(),"owners":{"rel":owner("rel"),"phy":owner("phy")},"entity_mentions":{"total":mentions.iter().sum::<usize>(),"average":ratio(mentions.iter().sum::<usize>() as u64, rows.len() as u64),"zero_cases":mentions.iter().filter(|&&n| n==0).count(),"max":mentions.iter().copied().max().unwrap_or(0),"at_limit":mentions.iter().filter(|&&n| n>=64).count()}})
}

fn endpoint(
    repo: &Path,
    effort: ModelReasoningEffort,
    model: &str,
) -> Result<(ConfiguredGeneralEndpoint, String), Box<dyn Error>> {
    let config = ReliquaryConfig::open(repo.join("reliquary.cfg"))?;
    let mut models = config.models.clone();
    let route = models
        .insomnia_metadata
        .as_mut()
        .ok_or("distribution calibration requires insomnia_metadata route")?;
    if route.provider != ModelProvider::OpenAiCodex {
        return Err("Luna calibration requires openai-codex metadata route".into());
    }
    route.model = model.into();
    route.reasoning_effort = Some(effort);
    let switchboard = ModelSwitchboard::new(models, config.credentials.clone())?;
    let endpoint = ConfiguredGeneralEndpoint::from_insomnia_metadata_switchboard(&switchboard)?;
    let model = endpoint.model().to_owned();
    Ok((endpoint, model))
}

fn parse_effort(v: &str) -> Result<ModelReasoningEffort, Box<dyn Error>> {
    match v {
        "low" => Ok(ModelReasoningEffort::Low),
        "medium" => Ok(ModelReasoningEffort::Medium),
        "high" => Ok(ModelReasoningEffort::High),
        _ => Err(format!("unsupported reasoning effort: {v}").into()),
    }
}
fn parse_usize(v: Option<&String>, default: usize, min: usize, max: usize) -> usize {
    v.and_then(|s| s.parse().ok())
        .unwrap_or(default)
        .clamp(min, max)
}
fn ratio(part: u64, total: u64) -> f64 {
    if total == 0 {
        0.0
    } else {
        part as f64 / total as f64
    }
}
fn load_jsonl(path: impl AsRef<Path>) -> Result<Vec<Value>, Box<dyn Error>> {
    Ok(fs::read_to_string(path)?
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(serde_json::from_str)
        .collect::<Result<_, _>>()?)
}
fn write_jsonl(path: &Path, rows: &[Value]) -> Result<(), Box<dyn Error>> {
    let mut w = BufWriter::new(File::create(path)?);
    for row in rows {
        writeln!(w, "{row}")?;
    }
    Ok(())
}
