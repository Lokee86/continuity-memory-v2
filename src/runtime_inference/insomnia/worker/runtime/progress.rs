use super::{DrainCounters, DrainShared};
use crate::{
    DEFAULT_INSOMNIA_PROGRESS_INTERVAL_SECS, InsomniaProgressEvent, InsomniaProgressReporter,
    InsomniaStats, InsomniaWork,
};
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::{Duration, Instant};

pub(super) struct DrainProgress<'a> {
    reporter: Option<&'a dyn InsomniaProgressReporter>,
    total: usize,
    initial_complete: usize,
    initial_terminal: usize,
    started: Instant,
}

impl<'a> DrainProgress<'a> {
    pub(super) fn new(
        reporter: Option<&'a dyn InsomniaProgressReporter>,
        stats: InsomniaStats,
    ) -> Self {
        Self {
            reporter,
            total: stats.total,
            initial_complete: stats.complete,
            initial_terminal: stats.terminal,
            started: Instant::now(),
        }
    }

    pub(super) fn enabled(&self) -> bool {
        self.reporter.is_some()
    }

    pub(super) fn report(&self, event: InsomniaProgressEvent) {
        if let Some(reporter) = self.reporter {
            reporter.report(&event);
        }
    }

    pub(super) fn counts(&self, counters: &DrainCounters) -> (usize, usize, usize) {
        (
            self.total,
            self.initial_complete + counters.completed.load(Ordering::Relaxed),
            self.initial_terminal + counters.terminal.load(Ordering::Relaxed),
        )
    }

    pub(super) fn elapsed_ms(&self) -> u64 {
        self.started
            .elapsed()
            .as_millis()
            .try_into()
            .unwrap_or(u64::MAX)
    }
}

pub(super) fn elapsed_ms_between(started_at_ns: i64, ended_at_ns: i64) -> u64 {
    ended_at_ns
        .saturating_sub(started_at_ns)
        .max(0)
        .checked_div(1_000_000)
        .and_then(|value| u64::try_from(value).ok())
        .unwrap_or(u64::MAX)
}

#[allow(clippy::too_many_arguments)]
pub(super) fn report_retry(
    shared: &DrainShared<'_>,
    claim: &InsomniaWork,
    started_at_ns: i64,
    failed_at_ns: i64,
    retry_after_ns: i64,
    backpressure: bool,
    error: String,
    counters: &DrainCounters,
) {
    let (total, complete, terminal) = shared.progress.counts(counters);
    shared.progress.report(InsomniaProgressEvent::EpisodeRetry {
        episode_id: claim.episode_id,
        attempt: claim.attempt_count,
        elapsed_ms: elapsed_ms_between(started_at_ns, failed_at_ns),
        retry_after_ms: retry_after_ns
            .max(0)
            .checked_div(1_000_000)
            .and_then(|value| u64::try_from(value).ok())
            .unwrap_or(u64::MAX),
        backpressure,
        error,
        total,
        complete,
        terminal,
    });
}

pub(super) fn heartbeat_loop(
    shared: &DrainShared<'_>,
    counters: &DrainCounters,
    stop: &AtomicBool,
) {
    let mut next_report = Duration::from_secs(DEFAULT_INSOMNIA_PROGRESS_INTERVAL_SECS);
    while !stop.load(Ordering::SeqCst) {
        thread::sleep(Duration::from_secs(1));
        if stop.load(Ordering::SeqCst) || shared.progress.started.elapsed() < next_report {
            continue;
        }
        let (total, complete, terminal) = shared.progress.counts(counters);
        shared.progress.report(InsomniaProgressEvent::Heartbeat {
            elapsed_ms: shared.progress.elapsed_ms(),
            total,
            complete,
            terminal,
            active_workers: counters.active.load(Ordering::Relaxed),
            claimed_attempts: counters.claimed.load(Ordering::Relaxed),
            failed_attempts: counters.failed.load(Ordering::Relaxed),
        });
        next_report += Duration::from_secs(DEFAULT_INSOMNIA_PROGRESS_INTERVAL_SECS);
    }
}
