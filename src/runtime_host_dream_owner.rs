use super::{ReliquaryRuntimeHostError, Shared, operation};
use crate::{
    CompatibilityProfileId, DreamCandidateConfig, DreamCandidateSet, DreamPairClassification,
    DreamPairVerification, DreamVerificationPolicy, MemoryId,
};
use std::collections::HashMap;
use std::time::Instant;

pub(super) struct DreamSnapshot {
    pub(super) candidates: DreamCandidateSet,
    graph_version: u64,
}

pub(super) fn prepare_project(
    shared: &Shared,
    profile_id: CompatibilityProfileId,
    cooldowns: &HashMap<MemoryId, Instant>,
) -> Result<Option<DreamSnapshot>, ReliquaryRuntimeHostError> {
    let mut runtime = shared
        .runtime
        .lock()
        .map_err(|_| ReliquaryRuntimeHostError::LockPoisoned)?;
    let now = Instant::now();
    for id in runtime.cva.memory_ids() {
        if cooldowns.get(&id).is_some_and(|until| *until > now) {
            continue;
        }
        let memory = runtime.cva.memory(id).map_err(operation)?;
        if memory.archived || memory.lifecycle_state != "extracted" {
            continue;
        }
        let body_id = runtime.cva.memory_body_id(id).map_err(operation)?;
        if runtime
            .cva
            .memory_vector_location(profile_id, body_id)
            .is_none()
        {
            continue;
        }
        let candidates = runtime
            .cva
            .dream_candidates(profile_id, id, DreamCandidateConfig::default())
            .map_err(operation)?;
        return Ok(Some(DreamSnapshot {
            candidates,
            graph_version: runtime.cva.graph_version(),
        }));
    }
    Ok(None)
}

pub(super) fn prepare_user(
    shared: &Shared,
    profile_id: CompatibilityProfileId,
    cooldowns: &HashMap<MemoryId, Instant>,
) -> Result<Option<DreamSnapshot>, ReliquaryRuntimeHostError> {
    let mut phylactery = shared
        .phylactery
        .lock()
        .map_err(|_| ReliquaryRuntimeHostError::LockPoisoned)?;
    let Some(phy) = phylactery.as_mut() else {
        return Ok(None);
    };
    let now = Instant::now();
    for id in phy.memory_ids() {
        if cooldowns.get(&id).is_some_and(|until| *until > now) {
            continue;
        }
        let memory = phy.memory(id).map_err(operation)?;
        if memory.archived || memory.lifecycle_state != "extracted" {
            continue;
        }
        let body_id = phy.memory_body_id(id).map_err(operation)?;
        if phy.memory_vector_location(profile_id, body_id).is_none() {
            continue;
        }
        let candidates = phy
            .dream_candidates(profile_id, id, DreamCandidateConfig::default())
            .map_err(operation)?;
        return Ok(Some(DreamSnapshot {
            candidates,
            graph_version: phy.graph_version(),
        }));
    }
    Ok(None)
}

pub(super) fn commit_project(
    shared: &Shared,
    snapshot: &DreamSnapshot,
    evaluated: Vec<(DreamPairClassification, Option<DreamPairVerification>)>,
) -> Result<bool, ReliquaryRuntimeHostError> {
    let mut runtime = shared
        .runtime
        .lock()
        .map_err(|_| ReliquaryRuntimeHostError::LockPoisoned)?;
    if runtime.cva.graph_version() != snapshot.graph_version
        || !project_snapshot_is_current(&mut runtime.cva, &snapshot.candidates)?
    {
        return Ok(false);
    }
    let policy = DreamVerificationPolicy::default();
    for (classification, verification) in evaluated {
        let graph_version = runtime.cva.graph_version();
        runtime
            .cva
            .publish_dream_pair(
                &classification,
                verification.as_ref(),
                policy,
                graph_version,
            )
            .map_err(operation)?;
    }
    runtime
        .cva
        .reconcile_dream_lifecycle(snapshot.candidates.source.memory.id)
        .map_err(operation)?;
    runtime.cva.sync().map_err(operation)?;
    Ok(true)
}

pub(super) fn commit_user(
    shared: &Shared,
    snapshot: &DreamSnapshot,
    evaluated: Vec<(DreamPairClassification, Option<DreamPairVerification>)>,
) -> Result<bool, ReliquaryRuntimeHostError> {
    let mut phylactery = shared
        .phylactery
        .lock()
        .map_err(|_| ReliquaryRuntimeHostError::LockPoisoned)?;
    let Some(phy) = phylactery.as_mut() else {
        return Ok(false);
    };
    if phy.graph_version() != snapshot.graph_version
        || !user_snapshot_is_current(phy, &snapshot.candidates)?
    {
        return Ok(false);
    }
    let policy = DreamVerificationPolicy::default();
    for (classification, verification) in evaluated {
        let graph_version = phy.graph_version();
        phy.publish_dream_pair(
            &classification,
            verification.as_ref(),
            policy,
            graph_version,
        )
        .map_err(operation)?;
    }
    phy.reconcile_dream_lifecycle(snapshot.candidates.source.memory.id)
        .map_err(operation)?;
    phy.sync().map_err(operation)?;
    Ok(true)
}

fn project_snapshot_is_current(
    cva: &mut crate::Cva,
    set: &DreamCandidateSet,
) -> Result<bool, ReliquaryRuntimeHostError> {
    if cva
        .memory(set.source.memory.id)
        .map_err(operation)?
        .revision
        != set.source.memory.revision
    {
        return Ok(false);
    }
    for candidate in &set.candidates {
        if cva
            .memory(candidate.context.memory.id)
            .map_err(operation)?
            .revision
            != candidate.context.memory.revision
        {
            return Ok(false);
        }
    }
    Ok(true)
}

fn user_snapshot_is_current(
    phy: &mut crate::Phylactery,
    set: &DreamCandidateSet,
) -> Result<bool, ReliquaryRuntimeHostError> {
    if phy
        .memory(set.source.memory.id)
        .map_err(operation)?
        .revision
        != set.source.memory.revision
    {
        return Ok(false);
    }
    for candidate in &set.candidates {
        if phy
            .memory(candidate.context.memory.id)
            .map_err(operation)?
            .revision
            != candidate.context.memory.revision
        {
            return Ok(false);
        }
    }
    Ok(true)
}
