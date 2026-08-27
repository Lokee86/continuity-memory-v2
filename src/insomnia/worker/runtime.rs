use super::{InsomniaDrainResult, InsomniaWorkerConfig, InsomniaWorkerError};
use crate::insomnia::evidence::{hydrate_evidence_parts, plan_evidence};
use crate::insomnia::extraction::{InsomniaExtractionStage, InsomniaExtractor};
use crate::insomnia::processor::claimed_episode_input_parts;
use crate::insomnia::store::InsomniaStore;
use crate::lexical_index::LexicalIndex;
use crate::memory_store::MemoryStore;
use crate::{
    Archive, Container, Cva, GeneralEndpoint, InsomniaExtractionError, InsomniaWork, Phylactery,
};
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

mod outcome;

use outcome::{apply_success, record_failure, terminal_claim};

#[derive(Default)]
pub(super) struct DrainCounters {
    pub(super) active: AtomicUsize,
    pub(super) peak: AtomicUsize,
    pub(super) claimed: AtomicUsize,
    pub(super) completed: AtomicUsize,
    pub(super) failed: AtomicUsize,
    pub(super) terminal: AtomicUsize,
    pub(super) created: AtomicUsize,
    pub(super) existing: AtomicUsize,
    pub(super) user_created: AtomicUsize,
    pub(super) user_existing: AtomicUsize,
    pub(super) rejected: AtomicUsize,
    pub(super) evidence_turns: AtomicUsize,
}

pub(super) struct DrainShared<'a> {
    pub(super) archive: &'a Archive,
    pub(super) lexical_index: &'a LexicalIndex,
    pub(super) container: Mutex<&'a mut Container>,
    pub(super) memories: Mutex<&'a mut MemoryStore>,
    pub(super) insomnia: Mutex<&'a mut InsomniaStore>,
    pub(super) phylactery: Option<Mutex<&'a mut Phylactery>>,
}

enum ClaimState {
    Claimed(InsomniaWork),
    Idle,
    Done,
}

pub(super) fn drain<E: GeneralEndpoint>(
    cva: &mut Cva,
    extractor: &InsomniaExtractor<E>,
    config: &InsomniaWorkerConfig,
) -> Result<InsomniaDrainResult, InsomniaWorkerError> {
    drain_inner(cva, None, extractor, config)
}

pub(super) fn drain_routed<E: GeneralEndpoint>(
    cva: &mut Cva,
    phylactery: &mut Phylactery,
    extractor: &InsomniaExtractor<E>,
    config: &InsomniaWorkerConfig,
) -> Result<InsomniaDrainResult, InsomniaWorkerError> {
    drain_inner(cva, Some(phylactery), extractor, config)
}

fn drain_inner<E: GeneralEndpoint>(
    cva: &mut Cva,
    phylactery: Option<&mut Phylactery>,
    extractor: &InsomniaExtractor<E>,
    config: &InsomniaWorkerConfig,
) -> Result<InsomniaDrainResult, InsomniaWorkerError> {
    cva.lexical_index
        .ensure_current(&cva.archive, &mut cva.container)?;
    let shared = DrainShared {
        archive: &cva.archive,
        lexical_index: &cva.lexical_index,
        container: Mutex::new(&mut cva.container),
        memories: Mutex::new(&mut cva.memories),
        insomnia: Mutex::new(&mut cva.insomnia),
        phylactery: phylactery.map(Mutex::new),
    };
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
    Ok(InsomniaDrainResult {
        workers: config.workers,
        peak_active_workers: counters.peak.load(Ordering::Relaxed),
        claimed_attempts: counters.claimed.load(Ordering::Relaxed),
        completed_episodes: counters.completed.load(Ordering::Relaxed),
        failed_attempts: counters.failed.load(Ordering::Relaxed),
        terminal_episodes: counters.terminal.load(Ordering::Relaxed),
        memories_created: counters.created.load(Ordering::Relaxed),
        memories_existing: counters.existing.load(Ordering::Relaxed),
        user_memories_created: counters.user_created.load(Ordering::Relaxed),
        user_memories_existing: counters.user_existing.load(Ordering::Relaxed),
        rejected_candidates: counters.rejected.load(Ordering::Relaxed),
        evidence_turns: counters.evidence_turns.load(Ordering::Relaxed),
        ..InsomniaDrainResult::default()
    })
}

fn worker_loop<E: GeneralEndpoint>(
    shared: &DrainShared<'_>,
    extractor: &InsomniaExtractor<E>,
    config: &InsomniaWorkerConfig,
    counters: &DrainCounters,
    index: usize,
) -> Result<(), InsomniaWorkerError> {
    let worker_id = format!("{}-{}", config.worker_id_prefix.trim(), index + 1);
    loop {
        let started_at_ns = now_ns();
        let claim = match claim_next(shared, config, &worker_id, started_at_ns)? {
            ClaimState::Claimed(claim) => claim,
            ClaimState::Idle => {
                sleep_ns(config.poll_interval_ns);
                continue;
            }
            ClaimState::Done => return Ok(()),
        };
        let input = {
            let mut container = shared
                .container
                .lock()
                .map_err(|_| InsomniaWorkerError::LockPoisoned)?;
            claimed_episode_input_parts(shared.archive, &mut container, &claim, &config.scope)
        };
        let (episode, turns) = match input {
            Ok(input) => input,
            Err(error) => {
                terminal_claim(shared, &claim, started_at_ns, error.to_string(), counters)?;
                continue;
            }
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

fn claim_next(
    shared: &DrainShared<'_>,
    config: &InsomniaWorkerConfig,
    worker_id: &str,
    started_at_ns: i64,
) -> Result<ClaimState, InsomniaWorkerError> {
    // Global order for combined mutable access is Container -> semantic store.
    let mut container = shared
        .container
        .lock()
        .map_err(|_| InsomniaWorkerError::LockPoisoned)?;
    let mut insomnia = shared
        .insomnia
        .lock()
        .map_err(|_| InsomniaWorkerError::LockPoisoned)?;
    let claim = insomnia.claim_next(
        &mut container,
        worker_id,
        started_at_ns,
        config.lease_duration_ns,
    )?;
    match claim {
        Some(claim) => Ok(ClaimState::Claimed(claim)),
        None => {
            let stats = insomnia.stats();
            if stats.pending == 0 && stats.processing == 0 && stats.failed == 0 {
                Ok(ClaimState::Done)
            } else {
                Ok(ClaimState::Idle)
            }
        }
    }
}

fn run_extraction<E: GeneralEndpoint>(
    shared: &DrainShared<'_>,
    extractor: &InsomniaExtractor<E>,
    episode: &crate::Episode,
    turns: &[crate::ResolvedTurn],
) -> Result<crate::InsomniaExtraction, InsomniaExtractionError> {
    match extractor.start(episode, turns)? {
        InsomniaExtractionStage::Complete(extraction) => Ok(extraction),
        InsomniaExtractionStage::Evidence(round) => {
            let plans = plan_evidence(
                shared.archive,
                shared.lexical_index,
                episode,
                &round.requests,
            );
            let (results, evidence_turns) = {
                let mut container = shared.container.lock().map_err(|_| {
                    InsomniaExtractionError::InvalidOutput("CVA container lock poisoned".into())
                })?;
                hydrate_evidence_parts(shared.archive, &mut container, &plans)?
            };
            extractor.finish_evidence(episode, turns, round, results, evidence_turns)
        }
    }
}

pub(super) fn now_ns() -> i64 {
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
