use crate::args::InsomniaCommand;
use crate::util::hex32;
use anyhow::Result;
use reliquary_memory::{ConfiguredInsomniaOptions, ConfiguredRuntime};
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
            let report = runtime.run_insomnia_files(
                &cva,
                phy.as_deref(),
                ConfiguredInsomniaOptions {
                    workers,
                    scope,
                    embedding_batch_size,
                    embedding_concurrency,
                    existing_queue_only,
                },
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
