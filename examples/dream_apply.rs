use reliquary_memory::{
    CompatibilityProfileId, ConfiguredGeneralEndpoint, Cva, DEFAULT_DREAM_FRONTIER_SIZE,
    DEFAULT_DREAM_INFERENCE_CONCURRENCY, DreamCandidateConfig, DreamMemoryProcessOutcome,
    DreamProcessError, DreamProcessor, DreamVerificationPolicy, GeneralEndpoint, ModelSwitchboard,
    Phylactery, ReliquaryConfig,
};
use std::collections::HashSet;
use std::error::Error;
use std::path::PathBuf;

const PROGRESS_BATCH: usize = 25;

fn main() {
    if let Err(error) = run() {
        eprintln!("dream apply failed: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.len() != 3 {
        return Err("usage: dream_apply <config> <rel> <phy>".into());
    }
    let config = ReliquaryConfig::open(PathBuf::from(&args[0]))?;
    let switchboard = ModelSwitchboard::new(config.models, config.credentials)?;
    let endpoint = ConfiguredGeneralEndpoint::from_dream_switchboard(&switchboard)?;
    let processor = DreamProcessor::new(endpoint.clone(), endpoint);

    let rel = process_rel(&processor, PathBuf::from(&args[1]))?;
    let phy = process_phy(&processor, PathBuf::from(&args[2]))?;
    println!(
        "dream_done: rel_attempted={} rel_failed={} rel_remaining={} rel_relations={} phy_attempted={} phy_failed={} phy_remaining={} phy_relations={}",
        rel.0, rel.1, rel.2, rel.3, phy.0, phy.1, phy.2, phy.3
    );
    Ok(())
}

fn rel_profile(cva: &Cva) -> Result<CompatibilityProfileId, Box<dyn Error>> {
    single_profile(
        cva.memory_vector_infos()
            .into_iter()
            .map(|info| info.compatibility_profile_id),
        "REL",
    )
}

fn phy_profile(phy: &Phylactery) -> Result<CompatibilityProfileId, Box<dyn Error>> {
    single_profile(
        phy.memory_vector_infos()
            .into_iter()
            .map(|info| info.compatibility_profile_id),
        "PHY",
    )
}

fn single_profile(
    profiles: impl Iterator<Item = CompatibilityProfileId>,
    owner: &str,
) -> Result<CompatibilityProfileId, Box<dyn Error>> {
    let profiles: HashSet<_> = profiles.collect();
    if profiles.len() != 1 {
        return Err(format!(
            "{owner} expected one memory-vector profile, found {}",
            profiles.len()
        )
        .into());
    }
    Ok(*profiles.iter().next().unwrap())
}

fn process_rel<C, V>(
    processor: &DreamProcessor<C, V>,
    path: PathBuf,
) -> Result<(usize, usize, usize, usize), Box<dyn Error>>
where
    C: GeneralEndpoint,
    V: GeneralEndpoint,
{
    let mut cva = Cva::open(path)?;
    let profile = rel_profile(&cva)?;
    let pending = pending_rel(&mut cva)?;
    println!(
        "dream_rel: pending={} frontier={} inference_concurrency={}",
        pending.len(),
        DEFAULT_DREAM_FRONTIER_SIZE,
        DEFAULT_DREAM_INFERENCE_CONCURRENCY
    );

    let mut attempted = 0;
    let mut failed = 0;
    for batch in pending.chunks(PROGRESS_BATCH) {
        let ids: Vec<_> = batch.iter().map(|(_, id)| *id).collect();
        let outcomes = processor.process_memories_with_concurrency(
            &mut cva,
            profile,
            &ids,
            DreamCandidateConfig::default(),
            DreamVerificationPolicy::default(),
            DEFAULT_DREAM_FRONTIER_SIZE,
            DEFAULT_DREAM_INFERENCE_CONCURRENCY,
        )?;
        failed += report_failures("dream_rel_error", &outcomes);
        attempted += outcomes.len();
        cva.sync()?;
        println!(
            "dream_rel_progress: {}/{} failed={}",
            attempted,
            pending.len(),
            failed
        );
    }
    Ok((
        pending.len(),
        failed,
        pending_rel(&mut cva)?.len(),
        cva.graph_stats().active_relations,
    ))
}

fn process_phy<C, V>(
    processor: &DreamProcessor<C, V>,
    path: PathBuf,
) -> Result<(usize, usize, usize, usize), Box<dyn Error>>
where
    C: GeneralEndpoint,
    V: GeneralEndpoint,
{
    let mut phy = Phylactery::open(path)?;
    let profile = phy_profile(&phy)?;
    let pending = pending_phy(&mut phy)?;
    println!(
        "dream_phy: pending={} frontier={} inference_concurrency={}",
        pending.len(),
        DEFAULT_DREAM_FRONTIER_SIZE,
        DEFAULT_DREAM_INFERENCE_CONCURRENCY
    );

    let mut attempted = 0;
    let mut failed = 0;
    for batch in pending.chunks(PROGRESS_BATCH) {
        let ids: Vec<_> = batch.iter().map(|(_, id)| *id).collect();
        let outcomes = processor.process_phylactery_memories_with_concurrency(
            &mut phy,
            profile,
            &ids,
            DreamCandidateConfig::default(),
            DreamVerificationPolicy::default(),
            DEFAULT_DREAM_FRONTIER_SIZE,
            DEFAULT_DREAM_INFERENCE_CONCURRENCY,
        )?;
        failed += report_failures("dream_phy_error", &outcomes);
        attempted += outcomes.len();
        phy.sync()?;
        println!(
            "dream_phy_progress: {}/{} failed={}",
            attempted,
            pending.len(),
            failed
        );
    }
    Ok((
        pending.len(),
        failed,
        pending_phy(&mut phy)?.len(),
        phy.graph_stats().active_relations,
    ))
}

fn report_failures(prefix: &str, outcomes: &[DreamMemoryProcessOutcome]) -> usize {
    let mut failed = 0;
    for outcome in outcomes {
        if let Err(error) = &outcome.result {
            debug_assert!(matches!(
                error,
                DreamProcessError::Classification(_) | DreamProcessError::Verification(_)
            ));
            failed += 1;
            eprintln!("{prefix}: memory={:?} error={error}", outcome.memory_id);
        }
    }
    failed
}

fn pending_rel(cva: &mut Cva) -> Result<Vec<(i64, reliquary_memory::MemoryId)>, Box<dyn Error>> {
    let mut pending = Vec::new();
    for id in cva.memory_ids() {
        let memory = cva.memory(id)?;
        if !memory.archived && memory.lifecycle_state == "extracted" {
            pending.push((memory.created_at_ns, id));
        }
    }
    pending.sort_by_key(|(created, id)| (*created, id.0));
    Ok(pending)
}

fn pending_phy(
    phy: &mut Phylactery,
) -> Result<Vec<(i64, reliquary_memory::MemoryId)>, Box<dyn Error>> {
    let mut pending = Vec::new();
    for id in phy.memory_ids() {
        let memory = phy.memory(id)?;
        if !memory.archived && memory.lifecycle_state == "extracted" {
            pending.push((memory.created_at_ns, id));
        }
    }
    pending.sort_by_key(|(created, id)| (*created, id.0));
    Ok(pending)
}
