use crate::args::InsomniaCommand;
use crate::util::hex32;
use anyhow::Result;
use reliquary_memory::{
    ConfiguredGeneralEndpoint, Cva, EpisodeConfig, InsomniaExtractor, InsomniaWorkState,
    InsomniaWorkerConfig, ModelSwitchboard, OpenAiReadyEmbeddingEndpoint, Phylactery,
    ReliquaryConfig,
};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

pub fn run(config_path: &Path, command: InsomniaCommand) -> Result<()> {
    match command {
        InsomniaCommand::Run {
            cva,
            phy,
            workers,
            scope,
            embedding_batch_size,
            embedding_concurrency,
            existing_queue_only,
        } => run_file(
            config_path,
            &cva,
            phy.as_deref(),
            workers,
            scope,
            embedding_batch_size,
            embedding_concurrency,
            existing_queue_only,
        ),
    }
}

#[allow(clippy::too_many_arguments)]
fn run_file(
    config_path: &Path,
    cva_path: &Path,
    phy_path: Option<&Path>,
    workers: usize,
    scope: String,
    embedding_batch_size: usize,
    embedding_concurrency: usize,
    existing_queue_only: bool,
) -> Result<()> {
    let config = ReliquaryConfig::open(config_path)?;
    let switchboard = ModelSwitchboard::new(config.models, config.credentials)?;
    let general = ConfiguredGeneralEndpoint::from_insomnia_switchboard(&switchboard)?;
    let embedding = OpenAiReadyEmbeddingEndpoint::from_switchboard(&switchboard)?
        .with_batching(embedding_batch_size, embedding_concurrency)?;
    let mut extractor = if switchboard.insomnia_metadata().is_some() {
        InsomniaExtractor::new(general).with_metadata_endpoint(
            ConfiguredGeneralEndpoint::from_insomnia_metadata_switchboard(&switchboard)?,
        )
    } else {
        InsomniaExtractor::new(general)
    };
    if phy_path.is_some() {
        let ownership = if switchboard.insomnia_metadata().is_some() {
            ConfiguredGeneralEndpoint::from_insomnia_metadata_switchboard(&switchboard)?
        } else {
            ConfiguredGeneralEndpoint::from_insomnia_switchboard(&switchboard)?
        };
        extractor = extractor.with_ownership_endpoint(ownership);
    }
    let mut cva = Cva::open(cva_path)?;
    let mut phylactery = phy_path.map(Phylactery::open).transpose()?;

    let before = cva.insomnia_stats();
    if !existing_queue_only {
        cva.finalize_canonical_imports_and_queue(EpisodeConfig::default(), now_ns())?;
    }
    let queued = cva.insomnia_stats();
    let worker_config = InsomniaWorkerConfig {
        workers,
        scope,
        ..Default::default()
    };
    let drain = match phylactery.as_mut() {
        Some(phylactery) => {
            cva.drain_insomnia_backlog_routed(phylactery, &extractor, &embedding, worker_config)?
        }
        None => cva.drain_insomnia_backlog(&extractor, &embedding, worker_config)?,
    };
    cva.sync()?;
    if let Some(phylactery) = &phylactery {
        phylactery.sync()?;
    }
    let final_stats = cva.insomnia_stats();

    println!(
        "queue: before={} after_registration={} final_complete={} final_terminal={}",
        before.total, queued.total, final_stats.complete, final_stats.terminal
    );
    println!(
        "workers: configured={} peak_active={} claimed={} completed={} failed_attempts={} terminal={}",
        drain.workers,
        drain.peak_active_workers,
        drain.claimed_attempts,
        drain.completed_episodes,
        drain.failed_attempts,
        drain.terminal_episodes
    );
    println!(
        "memories: project_created={} project_existing={} user_created={} user_existing={} rejected={} evidence_turns={}",
        drain.memories_created,
        drain.memories_existing,
        drain.user_memories_created,
        drain.user_memories_existing,
        drain.rejected_candidates,
        drain.evidence_turns
    );
    match drain.memory_vector_profile_id {
        Some(profile_id) => println!(
            "memory_vectors: profile={} embedded={} already_present={} created_set={}",
            hex32(&profile_id.0),
            drain.memory_vectors_embedded,
            drain.memory_vectors_already_present,
            drain
                .memory_vector_set_id
                .map(|id| hex32(&id.0))
                .unwrap_or_else(|| "none".into())
        ),
        None => println!("memory_vectors: skipped (no project memories)"),
    }
    if phylactery.is_some() {
        match drain.user_memory_vector_profile_id {
            Some(profile_id) => println!(
                "user_memory_vectors: profile={} embedded={} already_present={} created_set={}",
                hex32(&profile_id.0),
                drain.user_memory_vectors_embedded,
                drain.user_memory_vectors_already_present,
                drain
                    .user_memory_vector_set_id
                    .map(|id| hex32(&id.0))
                    .unwrap_or_else(|| "none".into())
            ),
            None => println!("user_memory_vectors: skipped (no user memories)"),
        }
    }
    for episode in cva.episodes() {
        if let Some(work) = cva.insomnia_work(episode.id)
            && work.state == InsomniaWorkState::Terminal
        {
            println!(
                "terminal_error: episode={} error={}",
                hex32(&episode.id.0),
                work.last_error.as_deref().unwrap_or("<missing>")
            );
        }
    }
    Ok(())
}

fn now_ns() -> i64 {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    i64::try_from(nanos).unwrap_or(i64::MAX)
}
