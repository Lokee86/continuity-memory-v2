use super::perception_flow::{defer, finish, now_ns, pop_next, requeue, sort_key};
use super::perception_owner::{commit, owner_id, prepare, seed_runtime};
use super::perception_queue::PerceptionOwner;
use super::perception_reconciliation::{PerceptionReconciliationState, ReconcileIdle, retry_poll};
use super::{
    IDLE_POLL, ReliquaryRuntimeHostError, Shared, notify_work, operation, stopped,
    wait_for_work_timeout,
};
use crate::{
    EntityResolutionEngine, EntityResolutionPreparation, EntityResolverError, GeneralEndpoint,
    GeneralEndpointError,
};
use serde_json::Value;
use std::sync::Arc;

#[derive(Clone)]
struct SharedEndpoint(Arc<dyn GeneralEndpoint>);

impl GeneralEndpoint for SharedEndpoint {
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
    seed_runtime(&shared)?;

    let mut seen_epoch = 0_u64;
    let mut prefer_user = false;
    let mut reconciliation = PerceptionReconciliationState::default();
    loop {
        if stopped(&shared.signal)? {
            return Ok(());
        }
        let (endpoint, decision_endpoint) = {
            let routes = shared
                .routes
                .read()
                .map_err(|_| ReliquaryRuntimeHostError::LockPoisoned)?;
            (
                routes.entity_resolution(),
                routes.entity_resolution_decision(),
            )
        };
        let Some(endpoint) = endpoint else {
            seen_epoch = wait_for_work_timeout(&shared.signal, seen_epoch, IDLE_POLL)?;
            continue;
        };

        let reconciliation_outcome =
            reconciliation.reconcile_if_due(&shared, Arc::clone(&endpoint))?;
        match reconciliation_outcome {
            ReconcileIdle::Changed => continue,
            ReconcileIdle::Deferred | ReconcileIdle::Retry => {
                let delay = retry_poll(reconciliation_outcome).unwrap_or(IDLE_POLL);
                seen_epoch = wait_for_work_timeout(&shared.signal, seen_epoch, delay)?;
                continue;
            }
            ReconcileIdle::Stable | ReconcileIdle::Unchanged | ReconcileIdle::Busy => {}
        }

        let Some((owner, key)) = pop_next(&shared, prefer_user)? else {
            seen_epoch = wait_for_work_timeout(&shared.signal, seen_epoch, IDLE_POLL)?;
            continue;
        };
        prefer_user = owner == PerceptionOwner::Project;

        let preparation = match prepare(&shared, owner, key, now_ns()) {
            Ok(Some(value)) => value,
            Ok(None) => {
                finish(&shared, owner)?;
                continue;
            }
            Err(error) => {
                finish(&shared, owner)?;
                return Err(error);
            }
        };
        let prepared = match preparation {
            EntityResolutionPreparation::Complete(_) => {
                finish(&shared, owner)?;
                continue;
            }
            EntityResolutionPreparation::Ready(prepared) => prepared,
        };

        let engine = EntityResolutionEngine::new(SharedEndpoint(endpoint))
            .with_decision_endpoint(decision_endpoint);
        let evaluation = match engine.evaluate_prepared(&prepared) {
            Ok(value) => value,
            Err(EntityResolverError::Endpoint(_)) => {
                finish(&shared, owner)?;
                defer(&shared, owner, key)?;
                seen_epoch = wait_for_work_timeout(&shared.signal, seen_epoch, IDLE_POLL)?;
                continue;
            }
            Err(error) => {
                finish(&shared, owner)?;
                return Err(operation(error));
            }
        };

        let prepared_owner_id = prepared.owner_id.clone();
        let effects = match commit(&shared, owner, prepared, evaluation, now_ns()) {
            Ok(Some(value)) => value,
            Ok(None) => {
                finish(&shared, owner)?;
                if owner_id(&shared, owner)?.as_deref() == Some(prepared_owner_id.as_str()) {
                    requeue(&shared, owner, key)?;
                }
                continue;
            }
            Err(error) => {
                finish(&shared, owner)?;
                return Err(error);
            }
        };
        finish(&shared, owner)?;

        let mut wakes = effects.same_surface_wakes;
        wakes.extend(effects.candidate_wakes);
        wakes.retain(|wake| *wake != key);
        wakes.sort_by_key(sort_key);
        wakes.dedup();
        if !wakes.is_empty() {
            let mut queue = shared
                .perception_queue
                .lock()
                .map_err(|_| ReliquaryRuntimeHostError::LockPoisoned)?;
            queue.push_all(owner, wakes);
            drop(queue);
            notify_work(&shared.signal)?;
        }
    }
}
