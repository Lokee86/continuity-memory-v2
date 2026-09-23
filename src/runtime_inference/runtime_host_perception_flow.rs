use super::perception_queue::PerceptionOwner;
use super::{DREAM_RETRY_POLL, ReliquaryRuntimeHostError, Shared};
use crate::MemoryEntityMentionKey;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

pub(super) fn pop_next(
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

pub(super) fn finish(
    shared: &Shared,
    owner: PerceptionOwner,
) -> Result<(), ReliquaryRuntimeHostError> {
    shared
        .perception_queue
        .lock()
        .map_err(|_| ReliquaryRuntimeHostError::LockPoisoned)?
        .finish(owner);
    Ok(())
}

pub(super) fn defer(
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

pub(super) fn requeue(
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

pub(super) fn sort_key(key: &MemoryEntityMentionKey) -> ([u8; 32], u8, u32, u32) {
    (
        key.memory_id.0,
        key.field.tag(),
        key.start_byte,
        key.end_byte,
    )
}

pub(super) fn now_ns() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos().min(i64::MAX as u128) as i64)
        .unwrap_or(0)
}
