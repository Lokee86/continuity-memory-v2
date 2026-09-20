mod runner;
mod score;
use reliquary_memory::{
    ConfiguredGeneralEndpoint, ModelReasoningEffort, ModelSwitchboard, ReliquaryConfig,
};
use serde_json::Value;
use std::{
    error::Error,
    fs::{self, File},
    io::{BufWriter, Write},
    path::{Path, PathBuf},
};
fn main() -> Result<(), Box<dyn Error>> {
    let a: Vec<String> = std::env::args().skip(1).collect();
    if a.len() < 4 {
        return Err("usage: entity_disambiguation_experiment <first-pass-results.jsonl> <candidates.jsonl> <output-dir> <config-path> [workers] [low|medium|high] [model]".into());
    }
    let workers = a
        .get(4)
        .and_then(|v| v.parse().ok())
        .unwrap_or(8)
        .clamp(1, 16);
    let effort = match a.get(5).map(String::as_str).unwrap_or("low") {
        "low" => ModelReasoningEffort::Low,
        "medium" => ModelReasoningEffort::Medium,
        "high" => ModelReasoningEffort::High,
        _ => return Err("reasoning must be low, medium, or high".into()),
    };
    let model = a.get(6).map(String::as_str).unwrap_or("gpt-5.6-sol");
    let prior = load(&a[0])?;
    let corpus = load(&a[1])?
        .into_iter()
        .filter_map(|v| Some((v["memory_id"].as_str()?.to_owned(), v)))
        .collect();
    let config = ReliquaryConfig::open(Path::new(&a[3]))?;
    let mut models = config.models.clone();
    let route = models
        .entity_resolution
        .as_mut()
        .ok_or("config requires entity_resolution route")?;
    route.model = model.into();
    route.reasoning_effort = Some(effort);
    let switchboard = ModelSwitchboard::new(models, config.credentials.clone())?;
    let endpoint = ConfiguredGeneralEndpoint::from_entity_resolution_switchboard(&switchboard)?;
    let out = PathBuf::from(&a[2]);
    fs::create_dir_all(&out)?;
    let rows = runner::run(endpoint, prior, corpus, workers)?;
    write_jsonl(&out.join("results.jsonl"), &rows)?;
    let summary = score::summarize(&rows, model, effort.as_str());
    fs::write(
        out.join("summary.json"),
        serde_json::to_string_pretty(&summary)?,
    )?;
    println!("{}", serde_json::to_string_pretty(&summary)?);
    Ok(())
}
fn load(p: impl AsRef<Path>) -> Result<Vec<Value>, Box<dyn Error>> {
    Ok(fs::read_to_string(p)?
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(serde_json::from_str)
        .collect::<Result<_, _>>()?)
}
fn write_jsonl(p: &Path, rows: &[Value]) -> Result<(), Box<dyn Error>> {
    let mut w = BufWriter::new(File::create(p)?);
    for r in rows {
        writeln!(w, "{r}")?;
    }
    Ok(())
}
