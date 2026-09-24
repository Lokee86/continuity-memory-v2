use super::perception_flow::now_ns;
use super::perception_queue::PerceptionOwner;
use super::{ReliquaryRuntimeHostError, Shared, operation};
use crate::{
    EntityCandidateConfig, EntityResolutionEvaluation, EntityResolutionOutcome,
    EntityResolutionPreparation, EntityResolutionPrepared, MemoryEntityMentionKey,
};

pub(super) struct PerceptionCommitEffects {
    pub(super) same_surface_wakes: Vec<MemoryEntityMentionKey>,
    pub(super) candidate_wakes: Vec<MemoryEntityMentionKey>,
}

pub(super) fn seed_runtime(shared: &Shared) -> Result<(), ReliquaryRuntimeHostError> {
    let attached_principal = if shared.phylactery_active() {
        let slot = shared
            .phylactery
            .lock()
            .map_err(|_| ReliquaryRuntimeHostError::LockPoisoned)?;
        slot.as_ref()
            .and_then(|phy| phy.owner_id().map(|owner_id| (owner_id, phy.profile())))
    } else {
        None
    };
    let project_keys = {
        let mut runtime = shared
            .runtime
            .lock()
            .map_err(|_| ReliquaryRuntimeHostError::LockPoisoned)?;
        if let Some((principal_id, profile)) = attached_principal
            && runtime
                .cva
                .sync_principal_profile(&principal_id, &profile, now_ns())
                .map_err(operation)?
        {
            runtime.cva.sync().map_err(operation)?;
        }
        startup_keys_project(&mut runtime.cva)?
    };
    shared
        .perception_queue
        .lock()
        .map_err(|_| ReliquaryRuntimeHostError::LockPoisoned)?
        .push_all(PerceptionOwner::Project, project_keys);

    let user_keys = if shared.phylactery_active() {
        let mut slot = shared
            .phylactery
            .lock()
            .map_err(|_| ReliquaryRuntimeHostError::LockPoisoned)?;
        match slot.as_mut() {
            Some(phy) => startup_keys_user(phy)?,
            None => Vec::new(),
        }
    } else {
        Vec::new()
    };
    shared
        .perception_queue
        .lock()
        .map_err(|_| ReliquaryRuntimeHostError::LockPoisoned)?
        .push_all(PerceptionOwner::User, user_keys);
    Ok(())
}

pub(super) fn prepare(
    shared: &Shared,
    owner: PerceptionOwner,
    key: MemoryEntityMentionKey,
    now_ns: i64,
) -> Result<Option<EntityResolutionPreparation>, ReliquaryRuntimeHostError> {
    match owner {
        PerceptionOwner::Project => {
            let attached_principal = if shared.phylactery_active() {
                let slot = shared
                    .phylactery
                    .lock()
                    .map_err(|_| ReliquaryRuntimeHostError::LockPoisoned)?;
                slot.as_ref()
                    .and_then(|phy| phy.owner_id().map(|owner_id| (owner_id, phy.profile())))
            } else {
                None
            };
            let mut runtime = shared
                .runtime
                .lock()
                .map_err(|_| ReliquaryRuntimeHostError::LockPoisoned)?;
            if let Some(outcome) = runtime
                .cva
                .resolve_principal_entity_mention(key, now_ns)
                .map_err(operation)?
            {
                if let Some((principal_id, profile)) = attached_principal
                    && outcome.entity_id
                        == Some(crate::entity_principal::principal_entity_id(&principal_id))
                {
                    runtime
                        .cva
                        .sync_principal_profile(&principal_id, &profile, now_ns)
                        .map_err(operation)?;
                }
                runtime.cva.sync().map_err(operation)?;
                return Ok(Some(EntityResolutionPreparation::Complete(outcome)));
            }
            runtime
                .cva
                .prepare_entity_resolution(key, EntityCandidateConfig::default())
                .map(Some)
                .map_err(operation)
        }
        PerceptionOwner::User => {
            if !shared.phylactery_active() {
                return Ok(None);
            }
            let mut slot = shared
                .phylactery
                .lock()
                .map_err(|_| ReliquaryRuntimeHostError::LockPoisoned)?;
            let Some(phy) = slot.as_mut() else {
                return Ok(None);
            };
            if let Some(outcome) = phy
                .resolve_principal_entity_mention(key, now_ns)
                .map_err(operation)?
            {
                phy.sync().map_err(operation)?;
                return Ok(Some(EntityResolutionPreparation::Complete(outcome)));
            }
            phy.prepare_entity_resolution(key, EntityCandidateConfig::default())
                .map(Some)
                .map_err(operation)
        }
    }
}

pub(super) fn owner_id(
    shared: &Shared,
    owner: PerceptionOwner,
) -> Result<Option<String>, ReliquaryRuntimeHostError> {
    match owner {
        PerceptionOwner::Project => {
            let runtime = shared
                .runtime
                .lock()
                .map_err(|_| ReliquaryRuntimeHostError::LockPoisoned)?;
            Ok(runtime.cva.owner_id())
        }
        PerceptionOwner::User => {
            if !shared.phylactery_active() {
                return Ok(None);
            }
            let slot = shared
                .phylactery
                .lock()
                .map_err(|_| ReliquaryRuntimeHostError::LockPoisoned)?;
            Ok(slot.as_ref().and_then(crate::Phylactery::owner_id))
        }
    }
}

pub(super) fn commit(
    shared: &Shared,
    owner: PerceptionOwner,
    prepared: EntityResolutionPrepared,
    evaluation: EntityResolutionEvaluation,
    now_ns: i64,
) -> Result<Option<PerceptionCommitEffects>, ReliquaryRuntimeHostError> {
    let surface = prepared.candidates.mention.text.clone();
    match owner {
        PerceptionOwner::Project => {
            let mut runtime = shared
                .runtime
                .lock()
                .map_err(|_| ReliquaryRuntimeHostError::LockPoisoned)?;
            let Some(outcome) = runtime
                .cva
                .commit_entity_resolution(prepared, evaluation, now_ns)
                .map_err(operation)?
            else {
                return Ok(None);
            };
            runtime.cva.sync().map_err(operation)?;
            let candidate_wakes = candidate_wakes_project(&runtime.cva, &outcome);
            Ok(Some(PerceptionCommitEffects {
                same_surface_wakes: runtime
                    .cva
                    .pending_entity_mentions_for_surface(&surface)
                    .into_iter()
                    .filter(|key| *key != outcome.key)
                    .collect(),
                candidate_wakes,
            }))
        }
        PerceptionOwner::User => {
            if !shared.phylactery_active() {
                return Ok(None);
            }
            let mut slot = shared
                .phylactery
                .lock()
                .map_err(|_| ReliquaryRuntimeHostError::LockPoisoned)?;
            let Some(phy) = slot.as_mut() else {
                return Ok(None);
            };
            let Some(outcome) = phy
                .commit_entity_resolution(prepared, evaluation, now_ns)
                .map_err(operation)?
            else {
                return Ok(None);
            };
            phy.sync().map_err(operation)?;
            let candidate_wakes = candidate_wakes_user(phy, &outcome);
            Ok(Some(PerceptionCommitEffects {
                same_surface_wakes: phy
                    .pending_entity_mentions_for_surface(&surface)
                    .into_iter()
                    .filter(|key| *key != outcome.key)
                    .collect(),
                candidate_wakes,
            }))
        }
    }
}

fn startup_keys_project(
    cva: &mut crate::Cva,
) -> Result<Vec<MemoryEntityMentionKey>, ReliquaryRuntimeHostError> {
    let ids = cva.memory_ids();
    let mut keys = Vec::new();
    for id in ids {
        if !cva.dream_was_processed(id) {
            continue;
        }
        let memory = cva.memory(id).map_err(operation)?;
        if !memory.archived && memory.superseded_by.is_none() {
            keys.extend(cva.schedulable_entity_mentions_for_memory(id));
        }
    }
    Ok(keys)
}

pub(super) fn startup_keys_user(
    phy: &mut crate::Phylactery,
) -> Result<Vec<MemoryEntityMentionKey>, ReliquaryRuntimeHostError> {
    let ids = phy.memory_ids();
    let mut keys = Vec::new();
    for id in ids {
        if !phy.dream_was_processed(id) {
            continue;
        }
        let memory = phy.memory(id).map_err(operation)?;
        if !memory.archived && memory.superseded_by.is_none() {
            keys.extend(phy.schedulable_entity_mentions_for_memory(id));
        }
    }
    Ok(keys)
}

fn candidate_wakes_project(
    cva: &crate::Cva,
    outcome: &EntityResolutionOutcome,
) -> Vec<MemoryEntityMentionKey> {
    if !(outcome.entity_created || outcome.association_changed) {
        return Vec::new();
    }
    outcome
        .entity_id
        .map(|id| cva.pending_entity_mentions_for_candidate(id))
        .unwrap_or_default()
}

fn candidate_wakes_user(
    phy: &crate::Phylactery,
    outcome: &EntityResolutionOutcome,
) -> Vec<MemoryEntityMentionKey> {
    if !(outcome.entity_created || outcome.association_changed) {
        return Vec::new();
    }
    outcome
        .entity_id
        .map(|id| phy.pending_entity_mentions_for_candidate(id))
        .unwrap_or_default()
}
