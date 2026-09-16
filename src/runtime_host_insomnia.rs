use super::{
    ReliquaryRuntimeHostError, Shared, notify_work, operation, stopped, wait_for_work,
    wait_for_work_timeout,
};
use crate::insomnia::{InsomniaEvidenceRound, InsomniaExtractionStage, RuntimeInsomniaClaim};
use crate::{GeneralEndpoint, GeneralEndpointError, InsomniaExtractor};
use serde_json::Value;
use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::time::Duration;

#[derive(Clone)]
struct SharedGeneralEndpoint(Arc<dyn GeneralEndpoint>);

impl GeneralEndpoint for SharedGeneralEndpoint {
    fn model(&self) -> &str {
        self.0.model()
    }

    fn complete_json(
        &self,
        system_prompt: &str,
        user_payload: &str,
        schema_name: &str,
        schema: &Value,
    ) -> Result<Value, GeneralEndpointError> {
        self.0
            .complete_json(system_prompt, user_payload, schema_name, schema)
    }
}

pub(super) fn worker_loop(
    shared: Arc<Shared>,
    index: usize,
) -> Result<(), ReliquaryRuntimeHostError> {
    let worker_id = format!("{}-{}", shared.config.worker_id_prefix.trim(), index + 1);
    let mut seen_epoch = 0_u64;
    loop {
        if stopped(&shared.signal)? {
            return Ok(());
        }
        if !shared.insomnia_enabled.load(Ordering::SeqCst) {
            seen_epoch = wait_for_work(&shared.signal, seen_epoch)?;
            continue;
        }
        if index == 0 {
            sweep_inactive(&shared)?;
        }
        if let Some(wait) = backpressure_wait(&shared) {
            seen_epoch = wait_for_work_timeout(&shared.signal, seen_epoch, wait)?;
            continue;
        }
        let routes = shared
            .routes
            .read()
            .map_err(|_| ReliquaryRuntimeHostError::LockPoisoned)?
            .clone();
        let Some(main) = routes.insomnia() else {
            seen_epoch = wait_for_work(&shared.signal, seen_epoch)?;
            continue;
        };
        let mut extractor = InsomniaExtractor::new(SharedGeneralEndpoint(Arc::clone(&main)));
        if let Some(metadata) = routes.insomnia_metadata() {
            extractor = extractor.with_metadata_endpoint(SharedGeneralEndpoint(metadata));
        } else {
            extractor = extractor.with_enrichment_endpoint(SharedGeneralEndpoint(main));
        }
        if shared
            .phylactery
            .lock()
            .map_err(|_| ReliquaryRuntimeHostError::LockPoisoned)?
            .is_some()
            && let Some(ownership) = routes.insomnia_ownership()
        {
            extractor = extractor.with_ownership_endpoint(SharedGeneralEndpoint(ownership));
        }
        if let Some(temporal) = routes.chronos() {
            extractor = extractor.with_temporal_endpoint(SharedGeneralEndpoint(temporal));
        }
        let claim = {
            let mut runtime = shared
                .runtime
                .lock()
                .map_err(|_| ReliquaryRuntimeHostError::LockPoisoned)?;
            runtime
                .cva
                .claim_runtime_insomnia(&worker_id, &shared.config)
                .map_err(operation)?
        };
        if let Some(claim) = claim {
            process_claim(&shared, &extractor, claim)?;
            continue;
        }
        seen_epoch = wait_for_work(&shared.signal, seen_epoch)?;
    }
}

fn process_claim(
    shared: &Shared,
    extractor: &InsomniaExtractor<SharedGeneralEndpoint>,
    claim: RuntimeInsomniaClaim,
) -> Result<(), ReliquaryRuntimeHostError> {
    let extraction = match extractor.start(&claim.episode, &claim.turns) {
        Ok(InsomniaExtractionStage::Complete(extraction)) => Ok(extraction),
        Ok(InsomniaExtractionStage::Evidence(round)) => {
            finish_evidence(shared, extractor, &claim, round)
        }
        Err(error) => Err(error),
    };
    let extraction = match extraction {
        Ok(extraction) => extraction,
        Err(error) => return fail_claim(shared, &claim, &error),
    };
    let mut prepared = {
        let mut runtime = shared
            .runtime
            .lock()
            .map_err(|_| ReliquaryRuntimeHostError::LockPoisoned)?;
        runtime
            .cva
            .prepare_runtime_insomnia_application(&claim, extraction, &shared.config)
            .map_err(operation)?
    };
    if let Err(error) =
        crate::insomnia::temporal::infer_prepared(extractor.temporal_endpoint(), &mut prepared)
    {
        return fail_claim(shared, &claim, &error);
    }
    let mut runtime = shared
        .runtime
        .lock()
        .map_err(|_| ReliquaryRuntimeHostError::LockPoisoned)?;
    let mut phylactery = shared
        .phylactery
        .lock()
        .map_err(|_| ReliquaryRuntimeHostError::LockPoisoned)?;
    runtime
        .cva
        .commit_runtime_insomnia_prepared(phylactery.as_mut(), &claim, prepared, &shared.config)
        .map_err(operation)?;
    drop(phylactery);
    drop(runtime);
    notify_work(&shared.signal)
}

fn fail_claim(
    shared: &Shared,
    claim: &RuntimeInsomniaClaim,
    error: &crate::InsomniaExtractionError,
) -> Result<(), ReliquaryRuntimeHostError> {
    if let Some(retry_after_ns) = crate::insomnia::backpressure::retry_after_ns(error) {
        let until = crate::insomnia::runtime_step::now_ns().saturating_add(retry_after_ns);
        shared
            .insomnia_backpressure_until_ns
            .fetch_max(until, Ordering::SeqCst);
    }
    let mut runtime = shared
        .runtime
        .lock()
        .map_err(|_| ReliquaryRuntimeHostError::LockPoisoned)?;
    runtime
        .cva
        .fail_runtime_insomnia(claim, error, &shared.config)
        .map_err(operation)
}

fn backpressure_wait(shared: &Shared) -> Option<Duration> {
    let until = shared.insomnia_backpressure_until_ns.load(Ordering::SeqCst);
    let remaining = until.saturating_sub(crate::insomnia::runtime_step::now_ns());
    (remaining > 0).then(|| Duration::from_nanos(u64::try_from(remaining).unwrap_or(u64::MAX)))
}

fn finish_evidence(
    shared: &Shared,
    extractor: &InsomniaExtractor<SharedGeneralEndpoint>,
    claim: &RuntimeInsomniaClaim,
    round: InsomniaEvidenceRound,
) -> Result<crate::InsomniaExtraction, crate::InsomniaExtractionError> {
    let (results, evidence_turns) = {
        let mut runtime = shared.runtime.lock().map_err(|_| {
            crate::InsomniaExtractionError::InvalidOutput("runtime lock poisoned".into())
        })?;
        runtime
            .cva
            .hydrate_runtime_insomnia_evidence(claim, &round)?
    };
    extractor.finish_evidence(&claim.episode, &claim.turns, round, results, evidence_turns)
}

fn sweep_inactive(shared: &Shared) -> Result<(), ReliquaryRuntimeHostError> {
    let mut runtime = shared
        .runtime
        .lock()
        .map_err(|_| ReliquaryRuntimeHostError::LockPoisoned)?;
    let sessions: Vec<_> = runtime
        .sessions
        .iter()
        .filter(|(_, state)| state.leaf_message_id.is_some() && state.in_flight.is_none())
        .map(|(id, _)| id.clone())
        .collect();
    let now = crate::insomnia::runtime_step::now_ns();
    for session_id in sessions {
        runtime
            .finalize_inactive_session(&session_id, shared.episode_policy, now)
            .map_err(operation)?;
    }
    Ok(())
}
