use super::dream_owner::{commit_project, commit_user, prepare_project, prepare_user};
use super::{
    DREAM_RETRY_POLL, ReliquaryRuntimeHostError, Shared, notify_work, operation, stopped,
    wait_for_work,
};
use crate::{
    DreamProcessError, DreamProcessor, DreamVerificationPolicy, GeneralEndpoint,
    GeneralEndpointError, MemoryId,
};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

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

pub(super) fn worker_loop(shared: Arc<Shared>) -> Result<(), ReliquaryRuntimeHostError> {
    let mut seen_epoch = 0_u64;
    let mut project_cooldowns = HashMap::<MemoryId, Instant>::new();
    let mut user_cooldowns = HashMap::<MemoryId, Instant>::new();
    loop {
        if stopped(&shared.signal)? {
            return Ok(());
        }
        let endpoint = shared
            .routes
            .read()
            .map_err(|_| ReliquaryRuntimeHostError::LockPoisoned)?
            .dream();
        let profiles = *shared
            .memory_profiles
            .lock()
            .map_err(|_| ReliquaryRuntimeHostError::LockPoisoned)?;
        let Some(endpoint) = endpoint else {
            seen_epoch = wait_for_work(&shared.signal, seen_epoch)?;
            continue;
        };
        if profiles.project.is_none() && profiles.user.is_none() {
            seen_epoch = wait_for_work(&shared.signal, seen_epoch)?;
            continue;
        }

        let general = SharedGeneralEndpoint(endpoint);
        let processor = DreamProcessor::new(general.clone(), general);
        let mut attempted = false;

        if let Some(profile_id) = profiles.project
            && let Some(snapshot) = prepare_project(&shared, profile_id, &project_cooldowns)?
        {
            attempted = true;
            let source_id = snapshot.candidates.source.memory.id;
            match processor.evaluate_candidates(
                &snapshot.candidates,
                DreamVerificationPolicy::default(),
                1,
            ) {
                Ok(evaluated) => {
                    if commit_project(&shared, &snapshot, evaluated)? {
                        project_cooldowns.remove(&source_id);
                        notify_work(&shared.signal)?;
                    }
                }
                Err(DreamProcessError::Classification(_) | DreamProcessError::Verification(_)) => {
                    project_cooldowns.insert(source_id, Instant::now() + DREAM_RETRY_POLL);
                }
                Err(error) => return Err(operation(error)),
            }
        }

        if shared.phylactery_active()
            && let Some(profile_id) = profiles.user
            && let Some(snapshot) = prepare_user(&shared, profile_id, &user_cooldowns)?
        {
            attempted = true;
            let source_id = snapshot.candidates.source.memory.id;
            match processor.evaluate_candidates(
                &snapshot.candidates,
                DreamVerificationPolicy::default(),
                1,
            ) {
                Ok(evaluated) => {
                    if commit_user(&shared, &snapshot, evaluated)? {
                        user_cooldowns.remove(&source_id);
                        notify_work(&shared.signal)?;
                    }
                }
                Err(DreamProcessError::Classification(_) | DreamProcessError::Verification(_)) => {
                    user_cooldowns.insert(source_id, Instant::now() + DREAM_RETRY_POLL);
                }
                Err(error) => return Err(operation(error)),
            }
        }

        project_cooldowns.retain(|_, until| *until > Instant::now());
        user_cooldowns.retain(|_, until| *until > Instant::now());
        if !attempted {
            seen_epoch = wait_for_work(&shared.signal, seen_epoch)?;
        }
    }
}
