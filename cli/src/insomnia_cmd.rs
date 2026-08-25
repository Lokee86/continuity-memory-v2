use crate::args::InsomniaCommand;
use crate::util::hex32;
use anyhow::Result;
use reliquary_memory::{
    ConfiguredGeneralEndpoint, Cva, EpisodeConfig, InsomniaExtractor, InsomniaWorkState,
    InsomniaWorkerConfig, ModelSwitchboard, OpenAiReadyEmbeddingEndpoint, ReliquaryConfig,
};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

pub fn run(config_path: &Path, command: InsomniaCommand) -> Result<()> {
    match command {
        InsomniaCommand::Run {
            cva,
            workers,
            scope,
            embedding_batch_size,
            embedding_concurrency,
            existing_queue_only,
        } => run_file(
            config_path,
            &cva,
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
    let extractor = InsomniaExtractor::new(general);
    let mut cva = Cva::open(cva_path)?;

    let before = cva.insomnia_stats();
    if !existing_queue_only {
        cva.finalize_canonical_imports_and_queue(EpisodeConfig::default(), now_ns())?;
    }
    let queued = cva.insomnia_stats();
    let drain = cva.drain_insomnia_backlog(
        &extractor,
        &embedding,
        InsomniaWorkerConfig {
            workers,
            scope,
            ..Default::default()
        },
    )?;
    cva.sync()?;
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
        "memories: created={} existing={} rejected={} evidence_turns={}",
        drain.memories_created,
        drain.memories_existing,
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
        None => println!("memory_vectors: skipped (no memories)"),
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
