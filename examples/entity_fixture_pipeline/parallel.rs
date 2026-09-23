use crate::parse::{Owner, load_rows, ordered_mention_keys};
use reliquary_memory::{
    ConfiguredDecisionEndpoint, ConfiguredGeneralEndpoint, Cva, DecisionEndpoint,
    EntityCandidateConfig, EntityResolutionEngine, ModelSwitchboard, Phylactery, ReliquaryConfig,
};
use std::error::Error;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

const PROGRESS_BATCH: usize = 50;

pub fn resume_parallel_batch(
    config: &str,
    results: &str,
    rel: &str,
    phy: &str,
    limit: usize,
    workers: usize,
) -> Result<(), Box<dyn Error>> {
    let config = ReliquaryConfig::open(config)?;
    let switchboard = ModelSwitchboard::new(config.models, config.credentials)?;
    let fallback = ConfiguredGeneralEndpoint::from_entity_resolution_switchboard(&switchboard)?;
    let reconciliation_endpoint = fallback.clone();
    let decision = switchboard
        .entity_resolution_decision()
        .map(|_| {
            ConfiguredDecisionEndpoint::from_entity_resolution_decision_switchboard(&switchboard)
                .map(|endpoint| Arc::new(endpoint) as Arc<dyn DecisionEndpoint>)
        })
        .transpose()?;
    let engine = EntityResolutionEngine::new(fallback).with_decision_endpoint(decision);

    let rows = load_rows(results)?;
    let total_mentions = rows
        .iter()
        .map(|row| row.metadata.entity_mentions.len())
        .sum::<usize>();
    let mut rel = Cva::open(rel)?;
    let mut phy = Phylactery::open(phy)?;
    let ordered = ordered_mention_keys(&rows, &mut rel, &mut phy)?;
    let mut tasks = Vec::with_capacity(limit);

    for (owner, key) in ordered {
        if tasks.len() >= limit {
            break;
        }
        let unseen = match owner {
            Owner::Rel => rel.entity_resolution(key).is_none(),
            Owner::Phy => phy.entity_resolution(key).is_none(),
        };
        if unseen {
            tasks.push((owner, key));
        }
    }

    let starting_states = rel.entity_resolutions().len() + phy.entity_resolutions().len();
    println!(
        "parallel resolution start: durable_states={starting_states}/{total_mentions} selected={} workers={}",
        tasks.len(),
        workers.max(1)
    );

    let config = EntityCandidateConfig::default();
    let mut processed = 0usize;
    let mut speculative = 0usize;
    let mut reevaluated = 0usize;
    let mut base = now_ns();

    for chunk in tasks.chunks(PROGRESS_BATCH) {
        let rel_keys = chunk
            .iter()
            .filter_map(|(owner, key)| (*owner == Owner::Rel).then_some(*key))
            .collect::<Vec<_>>();
        let phy_keys = chunk
            .iter()
            .filter_map(|(owner, key)| (*owner == Owner::Phy).then_some(*key))
            .collect::<Vec<_>>();

        if !rel_keys.is_empty() {
            let batch = match rel.resolve_entity_mentions_with_engine_parallel(
                &engine, &rel_keys, config, base, workers,
            ) {
                Ok(batch) => batch,
                Err(error) => {
                    rel.sync()?;
                    phy.sync()?;
                    return Err(error.into());
                }
            };
            processed += batch.outcomes.len();
            speculative += batch.speculative_evaluations;
            reevaluated += batch.reevaluations;
            base = base.saturating_add(rel_keys.len() as i64);
        }
        if !phy_keys.is_empty() {
            let batch = match phy.resolve_entity_mentions_with_engine_parallel(
                &engine, &phy_keys, config, base, workers,
            ) {
                Ok(batch) => batch,
                Err(error) => {
                    rel.sync()?;
                    phy.sync()?;
                    return Err(error.into());
                }
            };
            processed += batch.outcomes.len();
            speculative += batch.speculative_evaluations;
            reevaluated += batch.reevaluations;
            base = base.saturating_add(phy_keys.len() as i64);
        }

        rel.sync()?;
        phy.sync()?;
        let durable = rel.entity_resolutions().len() + phy.entity_resolutions().len();
        println!(
            "parallel progress: corpus={durable}/{total_mentions} processed={processed}/{} speculative={speculative} reevaluated={reevaluated} rel_entities={} phy_entities={}",
            tasks.len(),
            rel.entity_stats().entities,
            phy.entity_stats().entities
        );
    }

    print_reconciliation(
        "REL",
        &rel.reconcile_entities(&reconciliation_endpoint, now_ns())?,
    );
    print_reconciliation(
        "PHY",
        &phy.reconcile_entities(&reconciliation_endpoint, now_ns())?,
    );
    Ok(())
}

fn print_reconciliation(label: &str, report: &reliquary_memory::EntityReconciliationReport) {
    println!(
        "entity reconciliation: owner={label} rounds={} candidates={} deterministic_merges={} model_merges={} rejected_reconsidered={} unresolved_pairs={} clean={} findings={}",
        report.rounds,
        report.candidate_pairs,
        report.deterministic_merges,
        report.model_merges,
        report.rejected_mentions_reconsidered,
        report.unresolved_pairs,
        report.final_audit.is_clean(),
        report.final_audit.finding_count(),
    );
}

fn now_ns() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|value| value.as_nanos().min(i64::MAX as u128) as i64)
        .unwrap_or_default()
}
