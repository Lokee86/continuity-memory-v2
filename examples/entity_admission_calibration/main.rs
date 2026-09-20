use reliquary_memory::{
    ConfiguredGeneralEndpoint, ENTITY_ADMISSION_SYSTEM_PROMPT, GeneralEndpoint, ModelSwitchboard,
    ReliquaryConfig, entity_admission_schema,
};
use serde_json::{Value, json};
use std::{
    error::Error,
    fs::{self, File},
    io::{BufWriter, Write},
    path::{Path, PathBuf},
};

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.len() < 3 {
        return Err(
            "usage: entity_admission_calibration <fixture.jsonl> <output-dir> <config-path>".into(),
        );
    }
    let fixture = load_jsonl(&args[0])?;
    let config = ReliquaryConfig::open(&args[2])?;
    let switchboard = ModelSwitchboard::new(config.models, config.credentials)?;
    let endpoint = ConfiguredGeneralEndpoint::from_entity_resolution_switchboard(&switchboard)?;
    let out = PathBuf::from(&args[1]);
    fs::create_dir_all(&out)?;

    let mut rows = Vec::with_capacity(fixture.len());
    let mut correct = 0usize;
    for case in fixture {
        let title = required(&case, "title")?;
        let content = required(&case, "content")?;
        let mention = required(&case, "mention")?;
        let expected = required(&case, "expected_decision")?;
        let start = content
            .find(mention)
            .ok_or_else(|| format!("mention {mention:?} absent from fixture content"))?;
        let payload = json!({
            "title": title,
            "content": content,
            "mention": {
                "field": "content",
                "start_byte": start,
                "end_byte": start + mention.len(),
                "text": mention,
            },
            "context_evidence": [],
        });
        let raw = endpoint.complete_json(
            ENTITY_ADMISSION_SYSTEM_PROMPT,
            &payload.to_string(),
            "entity_admission_v4",
            &entity_admission_schema(),
        )?;
        let actual = raw
            .get("decision")
            .and_then(Value::as_str)
            .unwrap_or("<missing>");
        let matched = actual == expected;
        correct += usize::from(matched);
        rows.push(json!({
            "label": case.get("label").cloned().unwrap_or(Value::Null),
            "title": title,
            "content": content,
            "mention": mention,
            "expected_decision": expected,
            "actual_decision": actual,
            "matched": matched,
            "reason": raw.get("reason").cloned().unwrap_or(Value::Null),
            "promotion_policy": raw.get("promotion_policy").cloned().unwrap_or(Value::Null),
            "entity_kind": raw.get("entity_kind").cloned().unwrap_or(Value::Null),
            "entity_summary": raw.get("entity_summary").cloned().unwrap_or(Value::Null),
            "raw": raw,
        }));
    }

    let summary = json!({
        "model": endpoint.model(),
        "cases": rows.len(),
        "correct": correct,
        "accuracy": if rows.is_empty() { 0.0 } else { correct as f64 / rows.len() as f64 },
        "create_new": rows.iter().filter(|row| row["actual_decision"] == "create_new").count(),
        "unresolved": rows.iter().filter(|row| row["actual_decision"] == "unresolved").count(),
        "reject": rows.iter().filter(|row| row["actual_decision"] == "reject").count(),
    });
    write_jsonl(&out.join("results.jsonl"), &rows)?;
    fs::write(
        out.join("summary.json"),
        serde_json::to_string_pretty(&summary)?,
    )?;
    println!("{}", serde_json::to_string_pretty(&summary)?);
    for row in rows.iter().filter(|row| row["matched"] == false) {
        println!(
            "MISS {}: expected={} actual={} reason={}",
            row["label"].as_str().unwrap_or("<unlabelled>"),
            row["expected_decision"].as_str().unwrap_or("?"),
            row["actual_decision"].as_str().unwrap_or("?"),
            row["reason"].as_str().unwrap_or("?")
        );
    }
    Ok(())
}

fn load_jsonl(path: impl AsRef<Path>) -> Result<Vec<Value>, Box<dyn Error>> {
    Ok(fs::read_to_string(path)?
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(serde_json::from_str)
        .collect::<Result<_, _>>()?)
}

fn required<'a>(value: &'a Value, key: &str) -> Result<&'a str, Box<dyn Error>> {
    value
        .get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| format!("fixture missing string {key}").into())
}

fn write_jsonl(path: &Path, rows: &[Value]) -> Result<(), Box<dyn Error>> {
    let mut writer = BufWriter::new(File::create(path)?);
    for row in rows {
        writeln!(writer, "{row}")?;
    }
    Ok(())
}
