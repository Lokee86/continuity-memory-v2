use super::{ReliquaryRuntimeHostError, Shared, notify_work, operation, stopped, wait_for_work};
use crate::insomnia::{InsomniaEvidenceRound, InsomniaExtractionStage, RuntimeInsomniaClaim};
use crate::{GeneralEndpoint, GeneralEndpointError, InsomniaExtractor};
use serde_json::Value;
use std::sync::Arc;

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
        if index == 0 {
            sweep_inactive(&shared)?;
        }
        let endpoint = shared
            .general_endpoint
            .read()
            .map_err(|_| ReliquaryRuntimeHostError::LockPoisoned)?
            .clone();
        let Some(endpoint) = endpoint else {
            seen_epoch = wait_for_work(&shared.signal, seen_epoch)?;
            continue;
        };
        let extractor = InsomniaExtractor::new(SharedGeneralEndpoint(endpoint));
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
    let mut runtime = shared
        .runtime
        .lock()
        .map_err(|_| ReliquaryRuntimeHostError::LockPoisoned)?;
    match extraction {
        Ok(extraction) => {
            runtime
                .cva
                .commit_runtime_insomnia(&claim, extraction, &shared.config)
                .map_err(operation)?;
            drop(runtime);
            notify_work(&shared.signal)
        }
        Err(error) => runtime
            .cva
            .fail_runtime_insomnia(&claim, &error, &shared.config)
            .map_err(operation),
    }
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
