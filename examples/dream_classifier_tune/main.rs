mod support;

use continuity_memory::{
    ConfiguredGeneralEndpoint, ContinuityConfig, Cva, DreamClassifier, DreamRelationKind,
    GeneralEndpoint, ModelSwitchboard, DREAM_CLASSIFIER_CONTRACT_VERSION,
};
use serde_json::{json, Value};
use std::env;
use std::error::Error;
use std::fs;
use std::path::PathBuf;
use std::thread;
use support::{classification_json, memory_id, summarize, CaseTask};

fn main() {
    if let Err(error) = run() {
        eprintln!("Dream classifier tuning failed: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.len() < 4 {
        return Err("usage: dream_classifier_tune <config> <baseline-cva> <fixture.json> <output.json> [--concurrency N]".into());
    }
    let config_path = PathBuf::from(&args[0]);
    let cva_path = PathBuf::from(&args[1]);
    let fixture_path = PathBuf::from(&args[2]);
    let output_path = PathBuf::from(&args[3]);
    let concurrency = option_usize(&args[4..], "--concurrency")?
        .unwrap_or(12)
        .max(1);

    let config = ContinuityConfig::open(&config_path)?;
    let switchboard = ModelSwitchboard::new(config.models, config.credentials)?;
    let endpoint = ConfiguredGeneralEndpoint::from_dream_switchboard(&switchboard)?;
    let model = endpoint.model().to_owned();
    let fixture: Value = serde_json::from_slice(&fs::read(&fixture_path)?)?;
    let cases = fixture["cases"]
        .as_array()
        .ok_or("fixture cases must be an array")?;

    let mut cva = Cva::open(&cva_path)?;
    let mut tasks = Vec::with_capacity(cases.len());
    for case in cases {
        let left_id = memory_id(required_str(&case["left"], "id")?)?;
        let right_id = memory_id(required_str(&case["right"], "id")?)?;
        tasks.push(CaseTask {
            id: required_str(case, "id")?.to_owned(),
            pattern: required_str(case, "pattern")?.to_owned(),
            confidence: required_str(case, "confidence")?.to_owned(),
            expected: required_str(case, "expected")?.to_owned(),
            left: cva.dream_memory_context(left_id)?,
            right: cva.dream_memory_context(right_id)?,
        });
    }

    let mut results = Vec::with_capacity(tasks.len());
    for chunk in tasks.chunks(concurrency) {
        let chunk_results = thread::scope(|scope| {
            let handles = chunk
                .iter()
                .cloned()
                .map(|task| {
                    let endpoint = endpoint.clone();
                    scope.spawn(move || classify_case(task, endpoint))
                })
                .collect::<Vec<_>>();
            handles
                .into_iter()
                .map(|handle| handle.join().expect("classifier worker panicked"))
                .collect::<Vec<_>>()
        });
        results.extend(chunk_results);
        println!("classified {}/{}", results.len(), tasks.len());
    }

    let summary = summarize(&results);
    let report = json!({
        "format": "dream-classifier-tuning-v1",
        "classifier_model": model,
        "classifier_contract": DREAM_CLASSIFIER_CONTRACT_VERSION,
        "baseline_cva": cva_path,
        "fixture": fixture_path,
        "concurrency": concurrency,
        "summary": summary,
        "cases": results,
    });
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&output_path, serde_json::to_vec_pretty(&report)?)?;
    println!(
        "done correct={}/{} related={}/{} unrelated={}/{} errors={}",
        report["summary"]["correct"],
        report["summary"]["total"],
        report["summary"]["related_correct"],
        report["summary"]["related_total"],
        report["summary"]["unrelated_correct"],
        report["summary"]["unrelated_total"],
        report["summary"]["errors"],
    );
    Ok(())
}

fn classify_case(task: CaseTask, endpoint: ConfiguredGeneralEndpoint) -> Value {
    let classifier = DreamClassifier::new(endpoint);
    match classifier.classify_pair(&task.left, &task.right) {
        Ok(classification) => {
            let actual_related = classification.relation != DreamRelationKind::None;
            let expected_related = task.expected == "related";
            json!({
                "id": task.id,
                "pattern": task.pattern,
                "confidence": task.confidence,
                "expected": task.expected,
                "correct": actual_related == expected_related,
                "classification": classification_json(&classification),
                "error": Value::Null,
            })
        }
        Err(error) => json!({
            "id": task.id,
            "pattern": task.pattern,
            "confidence": task.confidence,
            "expected": task.expected,
            "correct": false,
            "classification": Value::Null,
            "error": error.to_string(),
        }),
    }
}

fn option_usize(args: &[String], name: &str) -> Result<Option<usize>, Box<dyn Error>> {
    let Some(index) = args.iter().position(|arg| arg == name) else {
        return Ok(None);
    };
    let value = args
        .get(index + 1)
        .ok_or_else(|| format!("missing value for {name}"))?;
    Ok(Some(value.parse()?))
}

fn required_str<'a>(value: &'a Value, key: &str) -> Result<&'a str, Box<dyn Error>> {
    value[key]
        .as_str()
        .ok_or_else(|| format!("missing string field {key}").into())
}
