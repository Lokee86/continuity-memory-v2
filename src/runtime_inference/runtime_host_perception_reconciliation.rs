use super::perception_flow::now_ns;
use super::{
    DREAM_RETRY_POLL, IDLE_POLL, ReliquaryRuntimeHostError, Shared, notify_work, operation,
};
use crate::entity_reconciliation_staged::evaluate_reconciliation;
use crate::{EntityReconciliationError, GeneralEndpoint, GeneralEndpointError};
use serde_json::Value;
use std::sync::Arc;

type OwnerVersions = ((u64, u64, u64), Option<(u64, u64, u64)>);

#[derive(Default)]
pub(super) struct PerceptionReconciliationState {
    last_versions: Option<OwnerVersions>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum ReconcileIdle {
    Stable,
    Changed,
    Deferred,
    Retry,
    Unchanged,
    Busy,
}

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

impl PerceptionReconciliationState {
    pub(super) fn reconcile_if_due(
        &mut self,
        shared: &Shared,
        endpoint: Arc<dyn GeneralEndpoint>,
    ) -> Result<ReconcileIdle, ReliquaryRuntimeHostError> {
        if !perception_idle(shared)? {
            return Ok(ReconcileIdle::Busy);
        }
        let versions = owner_versions(shared)?;
        if self.last_versions == Some(versions) {
            return Ok(ReconcileIdle::Unchanged);
        }
        let outcome = reconcile_idle(shared, endpoint)?;
        if outcome == ReconcileIdle::Stable {
            self.last_versions = Some(owner_versions(shared)?);
        }
        Ok(outcome)
    }
}

pub(super) fn retry_poll(outcome: ReconcileIdle) -> Option<std::time::Duration> {
    match outcome {
        ReconcileIdle::Deferred => Some(DREAM_RETRY_POLL),
        ReconcileIdle::Retry => Some(IDLE_POLL),
        _ => None,
    }
}

fn perception_idle(shared: &Shared) -> Result<bool, ReliquaryRuntimeHostError> {
    shared
        .perception_queue
        .lock()
        .map(|queue| queue.counts() == (0, 0))
        .map_err(|_| ReliquaryRuntimeHostError::LockPoisoned)
}

fn owner_versions(shared: &Shared) -> Result<OwnerVersions, ReliquaryRuntimeHostError> {
    let project = {
        let runtime = shared
            .runtime
            .lock()
            .map_err(|_| ReliquaryRuntimeHostError::LockPoisoned)?;
        (
            runtime.cva.entity_version(),
            runtime.cva.graph_version(),
            runtime.cva.memory_version(),
        )
    };
    let user = if shared.phylactery_active() {
        let slot = shared
            .phylactery
            .lock()
            .map_err(|_| ReliquaryRuntimeHostError::LockPoisoned)?;
        slot.as_ref().map(|phy| {
            (
                phy.entity_version(),
                phy.graph_version(),
                phy.memory_version(),
            )
        })
    } else {
        None
    };
    Ok((project, user))
}

fn reconcile_idle(
    shared: &Shared,
    endpoint: Arc<dyn GeneralEndpoint>,
) -> Result<ReconcileIdle, ReliquaryRuntimeHostError> {
    let project_prepared = {
        let mut runtime = shared
            .runtime
            .lock()
            .map_err(|_| ReliquaryRuntimeHostError::LockPoisoned)?;
        runtime
            .cva
            .prepare_entity_reconciliation()
            .map_err(operation)?
    };
    let user_prepared = if shared.phylactery_active() {
        let mut slot = shared
            .phylactery
            .lock()
            .map_err(|_| ReliquaryRuntimeHostError::LockPoisoned)?;
        slot.as_mut()
            .map(|phy| phy.prepare_entity_reconciliation())
            .transpose()
            .map_err(operation)?
    } else {
        None
    };

    let endpoint = SharedEndpoint(endpoint);
    let project_evaluation = match evaluate_reconciliation(&endpoint, &project_prepared) {
        Ok(value) => value,
        Err(EntityReconciliationError::Endpoint(_)) => return Ok(ReconcileIdle::Deferred),
        Err(error) => return Err(operation(error)),
    };
    let user_evaluation = match user_prepared.as_ref() {
        Some(prepared) => match evaluate_reconciliation(&endpoint, prepared) {
            Ok(value) => Some(value),
            Err(EntityReconciliationError::Endpoint(_)) => return Ok(ReconcileIdle::Deferred),
            Err(error) => return Err(operation(error)),
        },
        None => None,
    };

    let project_commit = {
        let mut runtime = shared
            .runtime
            .lock()
            .map_err(|_| ReliquaryRuntimeHostError::LockPoisoned)?;
        runtime
            .cva
            .commit_entity_reconciliation(project_prepared, project_evaluation, now_ns())
            .map_err(operation)?
    };
    let Some(project_commit) = project_commit else {
        return Ok(ReconcileIdle::Retry);
    };

    let had_user = user_prepared.is_some();
    let user_commit = match (user_prepared, user_evaluation) {
        (Some(prepared), Some(evaluation)) => {
            if !shared.phylactery_active() {
                return Ok(ReconcileIdle::Retry);
            }
            let mut slot = shared
                .phylactery
                .lock()
                .map_err(|_| ReliquaryRuntimeHostError::LockPoisoned)?;
            let Some(phy) = slot.as_mut() else {
                return Ok(ReconcileIdle::Retry);
            };
            phy.commit_entity_reconciliation(prepared, evaluation, now_ns())
                .map_err(operation)?
        }
        _ => None,
    };
    if had_user && user_commit.is_none() {
        return Ok(ReconcileIdle::Retry);
    }

    let changed = project_commit.report.changed()
        || user_commit
            .as_ref()
            .is_some_and(|commit| commit.report.changed());
    if changed {
        super::perception_owner::seed_runtime(shared)?;
        notify_work(&shared.signal)?;
        Ok(ReconcileIdle::Changed)
    } else {
        Ok(ReconcileIdle::Stable)
    }
}
