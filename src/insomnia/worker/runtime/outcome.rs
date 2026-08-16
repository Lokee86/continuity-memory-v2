use super::{DrainCounters, DrainShared, now_ns};
use crate::insomnia::processor::{commit_application, prepare_application};
use crate::{GeneralEndpointError, InsomniaError, InsomniaExtractionError, InsomniaWork};
use std::sync::atomic::Ordering;

#[allow(clippy::too_many_arguments)]
pub(super) fn apply_success(
    shared: &DrainShared<'_>,
    claim: &InsomniaWork,
    episode: &crate::Episode,
    turns: &[crate::ResolvedTurn],
    extraction: crate::InsomniaExtraction,
    started_at_ns: i64,
    config: &crate::InsomniaWorkerConfig,
    counters: &DrainCounters,
) -> Result<(), crate::InsomniaWorkerError> {
    counters
        .evidence_turns
        .fetch_add(extraction.evidence_turns.len(), Ordering::Relaxed);
    let completed_at_ns = now_ns();
    {
        let mut container = shared
            .container
            .lock()
            .map_err(|_| crate::InsomniaWorkerError::LockPoisoned)?;
        let mut insomnia = shared
            .insomnia
            .lock()
            .map_err(|_| crate::InsomniaWorkerError::LockPoisoned)?;
        insomnia.renew(
            &mut container,
            claim.episode_id,
            claim.lease_token.unwrap(),
            completed_at_ns,
            config.lease_duration_ns,
        )?;
    }
    let prepared = {
        let mut container = shared
            .container
            .lock()
            .map_err(|_| crate::InsomniaWorkerError::LockPoisoned)?;
        prepare_application(
            shared.archive,
            &mut container,
            episode,
            turns,
            extraction,
            &config.scope,
            completed_at_ns,
        )?
    };
    let result = {
        let mut container = shared
            .container
            .lock()
            .map_err(|_| crate::InsomniaWorkerError::LockPoisoned)?;
        let mut memories = shared
            .memories
            .lock()
            .map_err(|_| crate::InsomniaWorkerError::LockPoisoned)?;
        let mut insomnia = shared
            .insomnia
            .lock()
            .map_err(|_| crate::InsomniaWorkerError::LockPoisoned)?;
        commit_application(
            &mut container,
            &mut memories,
            &mut insomnia,
            claim,
            prepared,
            started_at_ns,
            completed_at_ns,
        )?
    };
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

pub(super) fn record_failure(
    shared: &DrainShared<'_>,
    claim: &InsomniaWork,
    started_at_ns: i64,
    error: InsomniaExtractionError,
    config: &crate::InsomniaWorkerConfig,
    counters: &DrainCounters,
) -> Result<(), crate::InsomniaWorkerError> {
    let failed_at_ns = now_ns();
    let reason = bounded_reason(error.to_string());
    if retryable(&error) && claim.attempt_count < config.max_attempts {
        let mut container = shared
            .container
            .lock()
            .map_err(|_| crate::InsomniaWorkerError::LockPoisoned)?;
        let mut insomnia = shared
            .insomnia
            .lock()
            .map_err(|_| crate::InsomniaWorkerError::LockPoisoned)?;
        insomnia.fail(
            &mut container,
            claim.episode_id,
            claim.lease_token.unwrap(),
            started_at_ns,
            failed_at_ns,
            failed_at_ns.saturating_add(config.retry_delay_ns),
            reason,
        )?;
        counters.failed.fetch_add(1, Ordering::Relaxed);
    } else {
        terminal_claim(shared, claim, started_at_ns, reason, counters)?;
    }
    Ok(())
}

pub(super) fn terminal_claim(
    shared: &DrainShared<'_>,
    claim: &InsomniaWork,
    started_at_ns: i64,
    reason: String,
    counters: &DrainCounters,
) -> Result<(), crate::InsomniaWorkerError> {
    let mut container = shared
        .container
        .lock()
        .map_err(|_| crate::InsomniaWorkerError::LockPoisoned)?;
    let mut insomnia = shared
        .insomnia
        .lock()
        .map_err(|_| crate::InsomniaWorkerError::LockPoisoned)?;
    insomnia.terminal(
        &mut container,
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
