mod admission;
mod admission_noul;
#[path = "../entity_resolution_experiment/score.rs"]
mod admission_score;
mod client;
mod common;
mod disambiguation;
#[path = "../entity_disambiguation_experiment/score.rs"]
mod disambiguation_score;

use client::JevClient;
use serde_json::Value;
use std::{
    collections::HashMap,
    error::Error,
    fs,
    path::{Path, PathBuf},
};

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.len() < 5 {
        return Err("usage: entity_jev_resolution_experiment <admission|admission-noul|disambiguation> <first-pass-results.jsonl> <candidates.jsonl> <output-dir> <config-path> [workers] [model]".into());
    }
    let mode = args[0].as_str();
    let prior = load(&args[1])?;
    let corpus = load(&args[2])?
        .into_iter()
        .filter_map(|v| Some((v["memory_id"].as_str()?.to_owned(), v)))
        .collect::<HashMap<_, _>>();
    let output = PathBuf::from(&args[3]);
    let workers = args
        .get(5)
        .and_then(|v| v.parse().ok())
        .unwrap_or(8usize)
        .clamp(1, 16);
    let model = args
        .get(6)
        .map(String::as_str)
        .unwrap_or("typesafe/jev-1.13");
    let client = JevClient::from_config(Path::new(&args[4]), model)?;
    fs::create_dir_all(&output)?;

    let rows = match mode {
        "admission" => admission::run(client, prior, corpus, workers),
        "admission-noul" => admission_noul::run(client, prior, corpus, workers),
        "disambiguation" => disambiguation::run(client, prior, corpus, workers),
        _ => return Err("mode must be admission, admission-noul, or disambiguation".into()),
    };
    write_jsonl(&output.join("results.jsonl"), &rows)?;
    let summary = match mode {
        "admission" | "admission-noul" => admission_score::summarize(&rows, model, "jev"),
        "disambiguation" => disambiguation_score::summarize(&rows, model, "jev"),
        _ => unreachable!(),
    };
    fs::write(
        output.join("summary.json"),
        serde_json::to_string_pretty(&summary)?,
    )?;
    println!("{}", serde_json::to_string_pretty(&summary)?);
    Ok(())
}

fn load(path: impl AsRef<Path>) -> Result<Vec<Value>, Box<dyn Error>> {
    Ok(fs::read_to_string(path)?
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(serde_json::from_str)
        .collect::<Result<_, _>>()?)
}

fn write_jsonl(path: &Path, rows: &[Value]) -> Result<(), Box<dyn Error>> {
    use std::io::Write;
    let mut writer = std::io::BufWriter::new(std::fs::File::create(path)?);
    for row in rows {
        writeln!(writer, "{row}")?;
    }
    Ok(())
}
