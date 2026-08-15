use super::{InsomniaDrainResult, InsomniaWorkerConfig, InsomniaWorkerError};
use crate::insomnia::evidence::resolve_evidence;
use crate::insomnia::extraction::{InsomniaExtractionStage, InsomniaExtractor};
use crate::{
    Cva, GeneralEndpoint, GeneralEndpointError, InsomniaError, InsomniaExtractionError,
    InsomniaWork,
};
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

#[derive(Default)]
struct DrainCounters {
    active: AtomicUsize,
    peak: AtomicUsize,
    claimed: AtomicUsize,
    completed: AtomicUsize,
    failed: AtomicUsize,
    terminal: AtomicUsize,
    created: AtomicUsize,
    existing: AtomicUsize,
    rejected: AtomicUsize,
    evidence_turns: AtomicUsize,
}

pub(super) fn drain<E: GeneralEndpoint>(
    cva: &mut Cva,
    extractor: &InsomniaExtractor<E>,
    config: &InsomniaWorkerConfig,
) -> Result<InsomniaDrainResult, InsomniaWorkerError> {
    let shared = Mutex::new(cva);
    let counters = DrainCounters::default();
    let results = thread::scope(|scope| {
        let mut handles = Vec::with_capacity(config.workers);
        for index in 0..config.workers {
            let shared = &shared;
            let counters = &counters;
            handles
                .push(scope.spawn(move || worker_loop(shared, extractor, config, counters, index)));
        }
        handles
            .into_iter()
            .map(|handle| handle.join())
            .collect::<Vec<_>>()
    });
    for result in results {
        match result {
            Ok(Ok(())) => {}
            Ok(Err(error)) => return Err(error),
            Err(_) => return Err(InsomniaWorkerError::ThreadPanicked),
        }
    }
    shared
        .into_inner()
        .map_err(|_| InsomniaWorkerError::LockPoisoned)?;
    Ok(InsomniaDrainResult {
        workers: config.workers,
        peak_active_workers: counters.peak.load(Ordering::Relaxed),
        claimed_attempts: counters.claimed.load(Ordering::Relaxed),
        completed_episodes: counters.completed.load(Ordering::Relaxed),
        failed_attempts: counters.failed.load(Ordering::Relaxed),
        terminal_episodes: counters.terminal.load(Ordering::Relaxed),
        memories_created: counters.created.load(Ordering::Relaxed),
        memories_existing: counters.existing.load(Ordering::Relaxed),
        rejected_candidates: counters.rejected.load(Ordering::Relaxed),
        evidence_turns: counters.evidence_turns.load(Ordering::Relaxed),
        ..InsomniaDrainResult::default()
    })
}

fn worker_loop<E: GeneralEndpoint>(
    shared: &Mutex<&mut Cva>,
    extractor: &InsomniaExtractor<E>,
    config: &InsomniaWorkerConfig,
    counters: &DrainCounters,
    index: usize,
) -> Result<(), InsomniaWorkerError> {
    let worker_id = format!("{}-{}", config.worker_id_prefix.trim(), index + 1);
    loop {
        let started_at_ns = now_ns();
        let claimed = {
            let mut cva = shared
                .lock()
                .map_err(|_| InsomniaWorkerError::LockPoisoned)?;
            let claim =
                cva.claim_insomnia_episode(&worker_id, started_at_ns, config.lease_duration_ns)?;
            match claim {
                Some(claim) => match cva.claimed_episode_input(&claim, &config.scope) {
                    Ok((episode, turns)) => Some((claim, episode, turns)),
                    Err(error) => {
                        terminal_claim(
                            &mut cva,
                            &claim,
                            started_at_ns,
                            error.to_string(),
                            counters,
                        )?;
                        None
                    }
                },
                None => {
                    let stats = cva.insomnia_stats();
                    if stats.pending == 0 && stats.processing == 0 && stats.failed == 0 {
                        return Ok(());
                    }
                    None
                }
            }
        };
        let Some((claim, episode, turns)) = claimed else {
            sleep_ns(config.poll_interval_ns);
            continue;
        };
        counters.claimed.fetch_add(1, Ordering::Relaxed);
        let active = counters.active.fetch_add(1, Ordering::SeqCst) + 1;
        counters.peak.fetch_max(active, Ordering::SeqCst);
        let extraction = run_extraction(shared, extractor, &episode, &turns);
        counters.active.fetch_sub(1, Ordering::SeqCst);
        match extraction {
            Ok(extraction) => apply_success(
                shared,
                &claim,
                &episode,
                &turns,
                extraction,
                started_at_ns,
                config,
                counters,
            )?,
            Err(error) => record_failure(shared, &claim, started_at_ns, error, config, counters)?,
        }
    }
}

fn run_extraction<E: GeneralEndpoint>(
    shared: &Mutex<&mut Cva>,
    extractor: &InsomniaExtractor<E>,
    episode: &crate::Episode,
    turns: &[crate::ResolvedTurn],
) -> Result<crate::InsomniaExtraction, InsomniaExtractionError> {
    match extractor.start(episode, turns)? {
        InsomniaExtractionStage::Complete(extraction) => Ok(extraction),
        InsomniaExtractionStage::Evidence(round) => {
            let (results, evidence_turns) = {
                let mut cva = shared.lock().map_err(|_| {
                    InsomniaExtractionError::InvalidOutput("CVA evidence lock poisoned".into())
                })?;
                resolve_evidence(&mut cva, episode, &round.requests)?
            };
            extractor.finish_evidence(episode, turns, round, results, evidence_turns)
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn apply_success(
    shared: &Mutex<&mut Cva>,
    claim: &InsomniaWork,
    episode: &crate::Episode,
    turns: &[crate::ResolvedTurn],
    extraction: crate::InsomniaExtraction,
    started_at_ns: i64,
    config: &InsomniaWorkerConfig,
    counters: &DrainCounters,
) -> Result<(), InsomniaWorkerError> {
    counters
        .evidence_turns
        .fetch_add(extraction.evidence_turns.len(), Ordering::Relaxed);
    let completed_at_ns = now_ns();
    let mut cva = shared
        .lock()
        .map_err(|_| InsomniaWorkerError::LockPoisoned)?;
    cva.renew_insomnia_lease(
        claim.episode_id,
        claim.lease_token.unwrap(),
        completed_at_ns,
        config.lease_duration_ns,
    )?;
    let result = cva.apply_claimed_insomnia_extraction(
        claim,
        episode,
        turns,
        extraction,
        &config.scope,
        started_at_ns,
        completed_at_ns,
    )?;
    counters.completed.fetch_add(1, Ordering::Relaxed);
    counters
        .created
        .fetch_add(result.created.len(), Ordering::Relaxed);
    counters
        .existing
        .fetch_add(result.existing.len(), Ordering::Relaxed);
    counters
        .rejected
        .fetch_add(result.rejected.len(), Ordering::Relaxed);
    Ok(())
}

fn record_failure(
    shared: &Mutex<&mut Cva>,
    claim: &InsomniaWork,
    started_at_ns: i64,
    error: InsomniaExtractionError,
    config: &InsomniaWorkerConfig,
    counters: &DrainCounters,
) -> Result<(), InsomniaWorkerError> {
    let failed_at_ns = now_ns();
    let reason = bounded_reason(error.to_string());
    let mut cva = shared
        .lock()
        .map_err(|_| InsomniaWorkerError::LockPoisoned)?;
    if retryable(&error) && claim.attempt_count < config.max_attempts {
        cva.fail_insomnia_episode(
            claim.episode_id,
            claim.lease_token.unwrap(),
            started_at_ns,
            failed_at_ns,
            failed_at_ns.saturating_add(config.retry_delay_ns),
            reason,
        )?;
        counters.failed.fetch_add(1, Ordering::Relaxed);
    } else {
        terminal_claim(&mut cva, claim, started_at_ns, reason, counters)?;
    }
    Ok(())
}

fn terminal_claim(
    cva: &mut Cva,
    claim: &InsomniaWork,
    started_at_ns: i64,
    reason: String,
    counters: &DrainCounters,
) -> Result<(), InsomniaWorkerError> {
    cva.terminal_insomnia_episode(
        claim.episode_id,
        claim.lease_token.ok_or(InsomniaError::InvalidLease)?,
        started_at_ns,
        now_ns(),
        bounded_reason(reason),
    )?;
    counters.terminal.fetch_add(1, Ordering::Relaxed);
    Ok(())
}

fn retryable(error: &InsomniaExtractionError) -> bool {
    !matches!(
        error,
        InsomniaExtractionError::Endpoint(GeneralEndpointError::InvalidConfiguration(_))
    )
}

fn bounded_reason(mut reason: String) -> String {
    reason = reason.trim().to_owned();
    if reason.is_empty() {
        return "Insomnia processing failed".into();
    }
    reason.truncate(4096);
    reason
}

fn now_ns() -> i64 {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    i64::try_from(nanos).unwrap_or(i64::MAX)
}

fn sleep_ns(value: i64) {
    thread::sleep(Duration::from_nanos(
        u64::try_from(value).unwrap_or(u64::MAX),
    ));
}
