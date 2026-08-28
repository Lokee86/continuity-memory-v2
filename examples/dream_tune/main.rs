mod report;
#[cfg(test)]
mod report_tests;
mod support;

use reliquary_memory::{
    ConfiguredGeneralEndpoint, Cva, DreamCandidateConfig, DreamProcessError, DreamProcessor,
    DreamVerificationPolicy, ModelSwitchboard, ReliquaryConfig,
};
use report::{candidates_json, durable_snapshot, id_hex, memory_json, result_json};
use serde_json::{Value, json};
use std::env;
use std::error::Error;
use std::fs;
use std::path::PathBuf;
use support::{
    candidate_config_json, option_string, option_usize, prepare_run_rel, single_memory_profile,
    write_gold_template, write_json, write_partial_report,
};

fn main() {
    if let Err(error) = run() {
        eprintln!("dream tuning harness failed: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.len() < 3 {
        return Err("usage: dream_tune <config> <baseline-rel-or-cva> <output-dir> [--limit N] [--source MEMORY_ID] [--candidate-limit N] [--pair-concurrency N] [--verification default|broad] [--retrieval-only] [--system-prompt-file PATH]".into());
    }
    let config_path = PathBuf::from(&args[0]);
    let baseline_path = PathBuf::from(&args[1]);
    let output_dir = PathBuf::from(&args[2]);
    let limit = option_usize(&args[3..], "--limit")?;
    let source_filter = option_string(&args[3..], "--source").map(str::to_owned);
    let candidate_limit = option_usize(&args[3..], "--candidate-limit")?;
    let pair_concurrency = option_usize(&args[3..], "--pair-concurrency")?
        .unwrap_or(12)
        .max(1);
    let retrieval_only = args[3..].iter().any(|arg| arg == "--retrieval-only");
    let verification = option_string(&args[3..], "--verification").unwrap_or("default");
    let system_prompt_file = option_string(&args[3..], "--system-prompt-file").map(PathBuf::from);
    let system_prompt = system_prompt_file
        .as_ref()
        .map(fs::read_to_string)
        .transpose()?;
    let verification_policy = match verification {
        "default" => DreamVerificationPolicy::default(),
        "broad" => DreamVerificationPolicy::broad_semantic(),
        other => return Err(format!("unsupported verification policy: {other}").into()),
    };

    let run_path = prepare_run_rel(&baseline_path, &output_dir)?;

    let config = ReliquaryConfig::open(&config_path)?;
    let switchboard = ModelSwitchboard::new(config.models, config.credentials)?;
    let endpoint = ConfiguredGeneralEndpoint::from_dream_switchboard(&switchboard)?;
    let processor = match system_prompt {
        Some(prompt) => {
            DreamProcessor::with_classifier_system_prompt(endpoint.clone(), endpoint, prompt)
        }
        None => DreamProcessor::new(endpoint.clone(), endpoint),
    };
    let classifier_model = processor.classifier_model().to_owned();
    let verifier_model = processor.verifier_model().to_owned();

    let mut cva = Cva::open(&run_path)?;
    let profile_id = single_memory_profile(&cva)?;
    let mut candidate_config = DreamCandidateConfig::default();
    if let Some(value) = candidate_limit {
        candidate_config.limit = value;
        candidate_config.prior_semantic_quota = candidate_config.prior_semantic_quota.min(value);
    }

    let baseline = durable_snapshot(&mut cva)?;
    let mut pending = Vec::new();
    for id in cva.memory_ids() {
        let memory = cva.memory(id)?;
        if !memory.archived && memory.lifecycle_state == "extracted" {
            pending.push((memory.created_at_ns, id));
        }
    }
    pending.sort_by_key(|(created_at_ns, id)| (*created_at_ns, id.0));
    if let Some(source) = source_filter.as_deref() {
        pending.retain(|(_, id)| id_hex(*id) == source);
        if pending.is_empty() {
            return Err(
                format!("requested source is not an active extracted Memory: {source}").into(),
            );
        }
    }
    if let Some(limit) = limit {
        pending.truncate(limit);
    }

    let mut runs = Vec::new();
    let mut recoverable_failures = 0usize;
    for (index, (_, source_id)) in pending.iter().copied().enumerate() {
        let source_before = cva.memory(source_id)?;
        let candidates = cva.dream_candidates(profile_id, source_id, candidate_config)?;
        let retrieval = candidates_json(&candidates);
        println!(
            "memory {}/{} {} candidates={} title={}",
            index + 1,
            pending.len(),
            &id_hex(source_id)[..12],
            candidates.candidates.len(),
            source_before.title
        );
        if retrieval_only {
            runs.push(json!({
                "source_before": memory_json(&source_before),
                "retrieval": retrieval,
                "result": Value::Null,
                "error": Value::Null,
            }));
            continue;
        }
        match processor.process_memory_with_pair_concurrency(
            &mut cva,
            profile_id,
            source_id,
            candidate_config,
            verification_policy,
            pair_concurrency,
        ) {
            Ok(result) => runs.push(json!({
                "source_before": memory_json(&source_before),
                "retrieval": retrieval,
                "result": result_json(&result),
                "error": Value::Null,
            })),
            Err(
                error @ (DreamProcessError::Classification(_) | DreamProcessError::Verification(_)),
            ) => {
                recoverable_failures += 1;
                runs.push(json!({
                    "source_before": memory_json(&source_before),
                    "retrieval": retrieval,
                    "result": Value::Null,
                    "error": error.to_string(),
                }));
            }
            Err(error) => {
                cva.sync()?;
                write_partial_report(
                    &output_dir,
                    &baseline_path,
                    &run_path,
                    &classifier_model,
                    &verifier_model,
                    candidate_config,
                    verification,
                    &baseline,
                    &runs,
                    recoverable_failures,
                    Some(error.to_string()),
                )?;
                return Err(error.into());
            }
        }
    }
    cva.sync()?;
    drop(cva);

    let mut reopened = Cva::open(&run_path)?;
    let durable = durable_snapshot(&mut reopened)?;
    let report = json!({
        "format": "dream-tuning-v1",
        "baseline_cva": baseline_path,
        "run_cva": run_path,
        "classifier_model": classifier_model,
        "verifier_model": verifier_model,
        "candidate_config": candidate_config_json(candidate_config),
        "verification_policy": verification,
        "pair_concurrency": pair_concurrency,
        "system_prompt_file": system_prompt_file,
        "retrieval_only": retrieval_only,
        "attempted": pending.len(),
        "recoverable_failures": recoverable_failures,
        "baseline": baseline,
        "runs": runs,
        "durable_after_reopen": durable,
    });
    write_json(&output_dir.join("report.json"), &report)?;
    write_gold_template(&output_dir, &report)?;
    println!(
        "done attempted={} failures={} remaining_extracted={} relations={}",
        pending.len(),
        recoverable_failures,
        report["durable_after_reopen"]["remaining_extracted"],
        report["durable_after_reopen"]["graph_stats"]["active_relations"]
    );
    Ok(())
}
