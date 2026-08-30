use crate::args::InsomniaCommand;
use crate::util::hex32;
use anyhow::Result;
use reliquary_memory::{ConfiguredInsomniaOptions, ConfiguredRuntime, InsomniaProgressEvent};
use std::io::{self, Write};
use std::path::Path;

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
        } => {
            let runtime = ConfiguredRuntime::open(config_path)?;
            println!(
                "progress: state=initializing rel={} phy={} workers={workers}",
                cva.display(),
                phy.as_deref()
                    .map(|path| path.display().to_string())
                    .unwrap_or_else(|| "none".into())
            );
            let _ = io::stdout().flush();
            let progress = |event: &InsomniaProgressEvent| print_progress(event);
            let report = runtime.run_insomnia_files_with_progress(
                &cva,
                phy.as_deref(),
                ConfiguredInsomniaOptions {
                    workers,
                    scope,
                    embedding_batch_size,
                    embedding_concurrency,
                    existing_queue_only,
                },
                &progress,
            )?;
            println!(
                "queue: before={} after_registration={} final_complete={} final_terminal={}",
                report.before.total,
                report.queued.total,
                report.final_stats.complete,
                report.final_stats.terminal
            );
            let drain = report.drain;
            println!(
                "workers: configured={} peak_active={} claimed={} completed={} failed_attempts={} terminal={} paused_for_backpressure={}",
                drain.workers,
                drain.peak_active_workers,
                drain.claimed_attempts,
                drain.completed_episodes,
                drain.failed_attempts,
                drain.terminal_episodes,
                drain.paused_for_backpressure
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
            if phy.is_some() {
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
            for failure in report.terminal_failures {
                println!(
                    "terminal_error: episode={} error={}",
                    hex32(&failure.episode_id.0),
                    failure.error
                );
            }
        }
    }
    Ok(())
}

fn print_progress(event: &InsomniaProgressEvent) {
    match event {
        InsomniaProgressEvent::DrainStarted {
            total,
            complete,
            terminal,
            workers,
        } => println!(
            "progress: state=started queue={complete}/{total} terminal={terminal} remaining={} workers={workers}",
            remaining(*total, *complete, *terminal)
        ),
        InsomniaProgressEvent::EpisodeStarted {
            episode_id,
            worker_id,
            attempt,
            turns,
            total,
            complete,
            terminal,
        } => println!(
            "progress: state=episode_started worker={worker_id} episode={} attempt={attempt} turns={turns} queue={complete}/{total} remaining={}",
            hex32(&episode_id.0),
            remaining(*total, *complete, *terminal)
        ),
        InsomniaProgressEvent::EpisodeStage {
            episode_id,
            worker_id,
            attempt,
            stage,
            items,
        } => println!(
            "progress: state=episode_stage worker={worker_id} episode={} attempt={attempt} stage={} items={items}",
            hex32(&episode_id.0),
            stage.as_str()
        ),
        InsomniaProgressEvent::Heartbeat {
            elapsed_ms,
            total,
            complete,
            terminal,
            active_workers,
            claimed_attempts,
            failed_attempts,
        } => println!(
            "progress: state=running elapsed={} active={active_workers} claimed={claimed_attempts} failed_attempts={failed_attempts} queue={complete}/{total} remaining={}",
            elapsed(*elapsed_ms),
            remaining(*total, *complete, *terminal)
        ),
        InsomniaProgressEvent::EpisodeCompleted {
            episode_id,
            attempt,
            elapsed_ms,
            total,
            complete,
            terminal,
            project_created,
            project_existing,
            user_created,
            user_existing,
            rejected,
            evidence_turns,
        } => println!(
            "progress: state=episode_complete episode={} attempt={attempt} elapsed={} queue={complete}/{total} remaining={} project_created={project_created} project_existing={project_existing} user_created={user_created} user_existing={user_existing} rejected={rejected} evidence_turns={evidence_turns}",
            hex32(&episode_id.0),
            elapsed(*elapsed_ms),
            remaining(*total, *complete, *terminal)
        ),
        InsomniaProgressEvent::EpisodeRetry {
            episode_id,
            attempt,
            elapsed_ms,
            retry_after_ms,
            backpressure,
            error,
            total,
            complete,
            terminal,
        } => println!(
            "progress: state={} episode={} attempt={attempt} elapsed={} retry_in={} queue={complete}/{total} remaining={} error={:?}",
            if *backpressure {
                "backpressure"
            } else {
                "retry"
            },
            hex32(&episode_id.0),
            elapsed(*elapsed_ms),
            elapsed(*retry_after_ms),
            remaining(*total, *complete, *terminal),
            compact_error(error)
        ),
        InsomniaProgressEvent::EpisodeTerminal {
            episode_id,
            attempt,
            elapsed_ms,
            error,
            total,
            complete,
            terminal,
        } => println!(
            "progress: state=terminal episode={} attempt={attempt} elapsed={} queue={complete}/{total} remaining={} error={:?}",
            hex32(&episode_id.0),
            elapsed(*elapsed_ms),
            remaining(*total, *complete, *terminal),
            compact_error(error)
        ),
        InsomniaProgressEvent::VectorizationStarted { owner, memories } => {
            println!("progress: state=vectorizing owner={owner} memories={memories}")
        }
        InsomniaProgressEvent::VectorizationCompleted {
            owner,
            embedded,
            already_present,
        } => println!(
            "progress: state=vectorization_complete owner={owner} embedded={embedded} already_present={already_present}"
        ),
    }
    let _ = io::stdout().flush();
}

fn remaining(total: usize, complete: usize, terminal: usize) -> usize {
    total.saturating_sub(complete.saturating_add(terminal))
}

fn elapsed(milliseconds: u64) -> String {
    format!("{:.1}s", milliseconds as f64 / 1000.0)
}

fn compact_error(error: &str) -> String {
    const LIMIT: usize = 512;
    let one_line = error.split_whitespace().collect::<Vec<_>>().join(" ");
    if one_line.chars().count() <= LIMIT {
        return one_line;
    }
    let mut value: String = one_line.chars().take(LIMIT).collect();
    value.push('…');
    value
}
