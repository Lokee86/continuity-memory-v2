use reliquary_memory::{
    ConfiguredGeneralEndpoint, GeneralEndpoint, ModelReasoningEffort, ModelSwitchboard,
    ReliquaryConfig, TemporalInferencer, chronos,
};
use serde_json::{Value, json};
use std::{error::Error, fs, path::PathBuf};

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.len() < 2 {
        return Err("usage: chronos_omnibus_replay <cases.jsonl> <output.json> [config]".into());
    }
    let config_path = args
        .get(2)
        .map(PathBuf::from)
        .unwrap_or_else(|| "reliquary.cfg".into());
    let mut config = ReliquaryConfig::open(config_path)?;
    let route = config
        .models
        .chronos
        .as_mut()
        .ok_or("config requires chronos route")?;
    route.model = "gpt-6-sol".into();
    route.reasoning_effort = Some(ModelReasoningEffort::Low);
    let switchboard = ModelSwitchboard::new(config.models, config.credentials)?;
    let endpoint = ConfiguredGeneralEndpoint::from_chronos_switchboard(&switchboard)?;
    let inferencer = TemporalInferencer::new(endpoint);

    let text = fs::read_to_string(&args[0])?;
    let mut rows = Vec::new();
    let mut inference_cases = 0usize;
    for line in text.lines().filter(|line| !line.trim().is_empty()) {
        let case: Value = serde_json::from_str(line)?;
        let semantic = format!(
            "{}\n{}",
            case["title"].as_str().unwrap_or(""),
            case["content"].as_str().unwrap_or("")
        );
        let source_time = case["source_time_ns"].as_i64();
        let assessment = chronos::assess(&semantic, source_time);
        if !assessment.resolution.needs_inference() {
            rows.push(json!({"memory_id":case["memory_id"],"needs_inference":false}));
            continue;
        }
        inference_cases += 1;
        match inferencer.infer(&semantic, source_time, &assessment) {
            Ok(inferred) => {
                let resolutions = inferred
                    .as_ref()
                    .map(|v| {
                        v.resolutions.iter().map(|r| json!({
                    "start_byte":r.start_byte,"end_byte":r.end_byte,"kind":format!("{:?}",r.kind),
                    "evidence":r.evidence,"canonical_expression":r.canonical_expression,
                })).collect::<Vec<_>>()
                    })
                    .unwrap_or_default();
                rows.push(json!({
                    "memory_id":case["memory_id"],"needs_inference":true,
                    "resolved_count":resolutions.len(),"resolutions":resolutions,"error":Value::Null,
                }));
            }
            Err(error) => rows.push(json!({
                "memory_id":case["memory_id"],"needs_inference":true,
                "resolved_count":0,"resolutions":[],"error":error.to_string(),
            })),
        }
    }
    let report = json!({
        "model":inferencer.model(),"cases":rows.len(),"inference_cases":inference_cases,"results":rows
    });
    if let Some(parent) = PathBuf::from(&args[1]).parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&args[1], serde_json::to_vec_pretty(&report)?)?;
    println!(
        "chronos verified {} inference cases with {}",
        inference_cases,
        inferencer.model()
    );
    Ok(())
}
