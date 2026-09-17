mod runner;
mod score;

use reliquary_memory::{
    ConfiguredGeneralEndpoint, GeneralEndpoint, ModelProvider, ModelReasoningEffort,
    ModelSwitchboard, ReliquaryConfig,
};
use serde_json::Value;
use std::collections::HashMap;
use std::error::Error;
use std::fs::{self, File};
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.len() < 3 {
        return Err(
            "usage: entity_calibration_run <gold.jsonl> <candidates.jsonl> <output-dir> [workers] [low|medium|high] [model]"
                .into(),
        );
    }
    let workers = args
        .get(3)
        .and_then(|v| v.parse().ok())
        .unwrap_or(8usize)
        .clamp(1, 16);
    let effort = args.get(4).map(String::as_str).unwrap_or("low");
    let effort = parse_reasoning_effort(effort)?;
    let model = args.get(5).map(String::as_str).unwrap_or("gpt-5.6-luna");
    let gold = load_jsonl(&args[0])?;
    let candidates = load_jsonl(&args[1])?
        .into_iter()
        .filter_map(|row| {
            let id = row["memory_id"].as_str()?.to_owned();
            Some((id, row))
        })
        .collect::<HashMap<_, _>>();
    let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let (endpoint, model) = endpoint(&repo, effort, model)?;
    let output = PathBuf::from(&args[2]);
    fs::create_dir_all(&output)?;

    let results = runner::run(endpoint, gold, candidates, workers)?;
    write_jsonl(&output.join("results.jsonl"), &results)?;
    let summary = score::summarize(&results, &model, effort.as_str());
    fs::write(
        output.join("summary.json"),
        serde_json::to_string_pretty(&summary)?,
    )?;
    println!("{}", serde_json::to_string_pretty(&summary)?);
    Ok(())
}

fn endpoint(
    repo: &Path,
    effort: ModelReasoningEffort,
    model: &str,
) -> Result<(ConfiguredGeneralEndpoint, String), Box<dyn Error>> {
    let config = ReliquaryConfig::open(repo.join("reliquary.cfg"))?;
    let mut models = config.models.clone();
    let route = if models.insomnia_metadata.is_some() {
        models.insomnia_metadata.as_mut().expect("checked")
    } else {
        return Err("Entity calibration requires the configured insomnia_metadata route".into());
    };
    if route.provider != ModelProvider::OpenAiCodex {
        return Err("Entity calibration requires openai-codex metadata route".into());
    }
    route.model = model.into();
    route.reasoning_effort = Some(effort);
    let switchboard = ModelSwitchboard::new(models, config.credentials.clone())?;
    let endpoint = ConfiguredGeneralEndpoint::from_insomnia_metadata_switchboard(&switchboard)?;
    let model = endpoint.model().to_owned();
    Ok((endpoint, model))
}

fn parse_reasoning_effort(value: &str) -> Result<ModelReasoningEffort, Box<dyn Error>> {
    match value {
        "low" => Ok(ModelReasoningEffort::Low),
        "medium" => Ok(ModelReasoningEffort::Medium),
        "high" => Ok(ModelReasoningEffort::High),
        other => Err(format!("unsupported reasoning effort: {other}").into()),
    }
}

fn load_jsonl(path: impl AsRef<Path>) -> Result<Vec<Value>, Box<dyn Error>> {
    Ok(fs::read_to_string(path)?
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(serde_json::from_str)
        .collect::<Result<_, _>>()?)
}

fn write_jsonl(path: &Path, rows: &[Value]) -> Result<(), Box<dyn Error>> {
    let mut writer = BufWriter::new(File::create(path)?);
    for row in rows {
        writeln!(writer, "{row}")?;
    }
    Ok(())
}
