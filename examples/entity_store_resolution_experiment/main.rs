mod runner;
mod score;
use reliquary_memory::{
    ConfiguredGeneralEndpoint, CredentialId, ModelReasoningEffort, ModelSwitchboard,
    ReliquaryConfig,
};
use serde_json::Value;
use std::{
    error::Error,
    fs,
    path::{Path, PathBuf},
};
fn main() -> Result<(), Box<dyn Error>> {
    let a: Vec<String> = std::env::args().skip(1).collect();
    if a.len() < 4 {
        return Err("usage: entity_store_resolution_experiment <queries.jsonl> <entities.jsonl> <output-dir> <config-path> [workers] [low|medium|high] [model] [credential-id]".into());
    }
    let workers = a
        .get(4)
        .and_then(|x| x.parse().ok())
        .unwrap_or(8)
        .clamp(1, 16);
    let effort = match a.get(5).map(String::as_str).unwrap_or("low") {
        "low" => ModelReasoningEffort::Low,
        "medium" => ModelReasoningEffort::Medium,
        "high" => ModelReasoningEffort::High,
        _ => return Err("reasoning must be low, medium, or high".into()),
    };
    let model = a.get(6).map(String::as_str).unwrap_or("gpt-5.6-sol");
    let config = ReliquaryConfig::open(Path::new(&a[3]))?;
    let mut models = config.models.clone();
    let route = models
        .entity_resolution
        .as_mut()
        .ok_or("config requires entity_resolution route")?;
    route.model = model.into();
    route.reasoning_effort = Some(effort);
    if let Some(credential_id) = a.get(7) {
        route.credential_id = CredentialId::new(credential_id.clone())?;
    }
    let ep = ConfiguredGeneralEndpoint::from_entity_resolution_switchboard(
        &ModelSwitchboard::new(models, config.credentials.clone())?,
    )?;
    let out = PathBuf::from(&a[2]);
    fs::create_dir_all(&out)?;
    let rows = runner::run(ep, load(&a[0])?, load(&a[1])?, workers)?;
    fs::write(
        out.join("results.jsonl"),
        rows.iter().map(|x| format!("{x}\n")).collect::<String>(),
    )?;
    let summary = score::summarize(&rows, model, effort.as_str());
    fs::write(
        out.join("summary.json"),
        serde_json::to_string_pretty(&summary)?,
    )?;
    println!("{summary}");
    Ok(())
}
fn load(p: impl AsRef<Path>) -> Result<Vec<Value>, Box<dyn Error>> {
    Ok(fs::read_to_string(p)?
        .lines()
        .filter(|x| !x.trim().is_empty())
        .map(serde_json::from_str)
        .collect::<Result<_, _>>()?)
}
