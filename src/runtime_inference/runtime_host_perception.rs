use super::perception_owner::{commit, owner_id, prepare, seed_runtime};
use super::perception_queue::PerceptionOwner;
use super::{
    DREAM_RETRY_POLL, IDLE_POLL, ReliquaryRuntimeHostError, Shared, notify_work, operation,
    stopped, wait_for_work_timeout,
};
use crate::{
    EntityResolutionPreparation, EntityResolver, EntityResolverError, GeneralEndpoint,
    GeneralEndpointError, MemoryEntityMentionKey,
};
use serde_json::Value;
use std::sync::Arc;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

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
    loop {
        if stopped(&shared.signal)? {
            return Ok(());
        }
        let endpoint = shared
            .routes
            .read()
            .map_err(|_| ReliquaryRuntimeHostError::LockPoisoned)?
            .perception();
        let Some(endpoint) = endpoint else {
            seen_epoch = wait_for_work_timeout(&shared.signal, seen_epoch, IDLE_POLL)?;
            continue;
        };

        let Some((owner, key)) = pop_next(&shared, prefer_user)? else {
            seen_epoch = wait_for_work_timeout(&shared.signal, seen_epoch, IDLE_POLL)?;
            continue;
        };
        prefer_user = owner == PerceptionOwner::Project;

        let preparation = match prepare(&shared, owner, key) {
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

        let resolver = EntityResolver::new(SharedEndpoint(endpoint));
        let evaluation = match resolver.evaluate_prepared(&prepared) {
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

fn pop_next(
    shared: &Shared,
    prefer_user: bool,
) -> Result<Option<(PerceptionOwner, MemoryEntityMentionKey)>, ReliquaryRuntimeHostError> {
    let mut queue = shared
        .perception_queue
        .lock()
        .map_err(|_| ReliquaryRuntimeHostError::LockPoisoned)?;
    let owners = if prefer_user {
        [PerceptionOwner::User, PerceptionOwner::Project]
    } else {
        [PerceptionOwner::Project, PerceptionOwner::User]
    };
    for owner in owners {
        if let Some(key) = queue.pop_ready(owner, Instant::now()) {
            return Ok(Some((owner, key)));
        }
    }
    Ok(None)
}

fn finish(shared: &Shared, owner: PerceptionOwner) -> Result<(), ReliquaryRuntimeHostError> {
    shared
        .perception_queue
        .lock()
        .map_err(|_| ReliquaryRuntimeHostError::LockPoisoned)?
        .finish(owner);
    Ok(())
}

fn defer(
    shared: &Shared,
    owner: PerceptionOwner,
    key: MemoryEntityMentionKey,
) -> Result<(), ReliquaryRuntimeHostError> {
    shared
        .perception_queue
        .lock()
        .map_err(|_| ReliquaryRuntimeHostError::LockPoisoned)?
        .defer(owner, key, Instant::now() + DREAM_RETRY_POLL);
    Ok(())
}

fn requeue(
    shared: &Shared,
    owner: PerceptionOwner,
    key: MemoryEntityMentionKey,
) -> Result<(), ReliquaryRuntimeHostError> {
    shared
        .perception_queue
        .lock()
        .map_err(|_| ReliquaryRuntimeHostError::LockPoisoned)?
        .push(owner, key);
    Ok(())
}

fn sort_key(key: &MemoryEntityMentionKey) -> ([u8; 32], u8, u32, u32) {
    (
        key.memory_id.0,
        key.field.tag(),
        key.start_byte,
        key.end_byte,
    )
}

fn now_ns() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos().min(i64::MAX as u128) as i64)
        .unwrap_or(0)
}
