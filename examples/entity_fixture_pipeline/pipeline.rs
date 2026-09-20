use crate::parse::{Owner, load_rows};
use reliquary_memory::{
    ConfiguredDecisionEndpoint, ConfiguredGeneralEndpoint, Cva, DecisionEndpoint,
    EntityCandidateConfig, EntityDraft, EntityId, EntityResolutionEngine, EntityResolutionOutcome,
    EntityResolverError, GeneralEndpointError, MemoryEntityMentionKey,
    MemoryEntityResolutionStatus, ModelSwitchboard, Phylactery, ReliquaryConfig,
};
use std::collections::HashSet;
use std::error::Error;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

pub fn import_extraction(results: &str, rel: &str, phy: &str) -> Result<(), Box<dyn Error>> {
    let rows = load_rows(results)?;
    let mut rel = Cva::open(rel)?;
    let mut phy = Phylactery::open(phy)?;
    let mut rel_changed = 0;
    let mut phy_changed = 0;
    let mut mentions = 0;
    for row in rows {
        mentions += row.metadata.entity_mentions.len();
        match row.owner {
            Owner::Rel => {
                if rel.memory_body_id(row.metadata.memory_id)? != row.metadata.body_id {
                    return Err("REL extraction body mismatch".into());
                }
                rel_changed += usize::from(rel.put_memory_routing_metadata(row.metadata)?);
            }
            Owner::Phy => {
                if phy.memory_body_id(row.metadata.memory_id)? != row.metadata.body_id {
                    return Err("PHY extraction body mismatch".into());
                }
                phy_changed += usize::from(phy.put_memory_routing_metadata(row.metadata)?);
            }
        }
    }
    rel.sync()?;
    phy.sync()?;
    println!(
        "imported routing metadata: rel_changed={rel_changed} phy_changed={phy_changed} mentions={mentions}"
    );
    Ok(())
}

pub fn resume(config: &str, results: &str, rel: &str, phy: &str) -> Result<(), Box<dyn Error>> {
    resolve_with_limit(
        config,
        results,
        rel,
        phy,
        None,
        ResolutionRunMode::UnseenOnly,
    )
}

pub fn resume_batch(
    config: &str,
    results: &str,
    rel: &str,
    phy: &str,
    limit: usize,
) -> Result<(), Box<dyn Error>> {
    resolve_with_limit(
        config,
        results,
        rel,
        phy,
        Some(limit),
        ResolutionRunMode::UnseenOnly,
    )
}

pub fn resolve_all(
    config: &str,
    results: &str,
    rel: &str,
    phy: &str,
) -> Result<(), Box<dyn Error>> {
    resolve_with_limit(
        config,
        results,
        rel,
        phy,
        None,
        ResolutionRunMode::PendingAndUnseen,
    )
}

pub fn resolve_batch(
    config: &str,
    results: &str,
    rel: &str,
    phy: &str,
    limit: usize,
) -> Result<(), Box<dyn Error>> {
    resolve_with_limit(
        config,
        results,
        rel,
        phy,
        Some(limit),
        ResolutionRunMode::PendingAndUnseen,
    )
}

pub fn resolve_target(
    config: &str,
    results: &str,
    rel: &str,
    phy: &str,
    memory_id_hex: &str,
    mention_text: &str,
) -> Result<(), Box<dyn Error>> {
    let config = ReliquaryConfig::open(config)?;
    let switchboard = ModelSwitchboard::new(config.models, config.credentials)?;
    let endpoint = ConfiguredGeneralEndpoint::from_entity_resolution_switchboard(&switchboard)?;
    let decision = switchboard
        .entity_resolution_decision()
        .map(|_| {
            ConfiguredDecisionEndpoint::from_entity_resolution_decision_switchboard(&switchboard)
                .map(|endpoint| Arc::new(endpoint) as Arc<dyn DecisionEndpoint>)
        })
        .transpose()?;
    let engine = EntityResolutionEngine::new(endpoint).with_decision_endpoint(decision);

    let rows = load_rows(results)?;
    let row = rows
        .into_iter()
        .find(|row| hex_memory_id(row.metadata.memory_id) == memory_id_hex)
        .ok_or("target memory not found in extraction results")?;
    let mention = row
        .metadata
        .entity_mentions
        .iter()
        .find(|mention| mention.text == mention_text)
        .ok_or("target mention not found in target memory")?;
    let key = MemoryEntityMentionKey::new(row.metadata.memory_id, mention);

    let mut rel = Cva::open(rel)?;
    let mut phy = Phylactery::open(phy)?;
    let existing = match row.owner {
        Owner::Rel => rel.entity_resolution(key).map(|value| value.status.clone()),
        Owner::Phy => phy.entity_resolution(key).map(|value| value.status.clone()),
    };
    if existing.is_some() {
        return Err("target mention already has durable resolution state".into());
    }

    let base = now_ns().saturating_add(1);
    let outcome = match row.owner {
        Owner::Rel => retry_resolution(|| {
            rel.resolve_entity_mention_with_engine(
                &engine,
                key,
                EntityCandidateConfig::default(),
                base,
            )
        })?,
        Owner::Phy => retry_resolution(|| {
            phy.resolve_entity_mention_with_engine(
                &engine,
                key,
                EntityCandidateConfig::default(),
                base,
            )
        })?,
    };

    rel.sync()?;
    phy.sync()?;
    let owner_label = match row.owner {
        Owner::Rel => "REL",
        Owner::Phy => "PHY",
    };
    println!(
        "target resolution: owner={} memory={} mention={:?} decision={:?} reason={:?} entity_id={:?} entity_created={} rel_entities={} phy_entities={} rel_states={} phy_states={}",
        owner_label,
        memory_id_hex,
        mention_text,
        outcome.decision,
        outcome.reason,
        outcome.entity_id,
        outcome.entity_created,
        rel.entity_stats().entities,
        phy.entity_stats().entities,
        rel.entity_resolutions().len(),
        phy.entity_resolutions().len(),
    );
    Ok(())
}

fn hex_memory_id(id: reliquary_memory::MemoryId) -> String {
    let mut out = String::with_capacity(64);
    for byte in id.0 {
        use std::fmt::Write;
        let _ = write!(&mut out, "{byte:02x}");
    }
    out
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ResolutionRunMode {
    UnseenOnly,
    PendingAndUnseen,
}

impl ResolutionRunMode {
    fn label(self) -> &'static str {
        match self {
            Self::UnseenOnly => "resume",
            Self::PendingAndUnseen => "resolve-all",
        }
    }
}

fn resolve_with_limit(
    config: &str,
    results: &str,
    rel: &str,
    phy: &str,
    limit: Option<usize>,
    mode: ResolutionRunMode,
) -> Result<(), Box<dyn Error>> {
    let config = ReliquaryConfig::open(config)?;
    let switchboard = ModelSwitchboard::new(config.models, config.credentials)?;
    let endpoint = ConfiguredGeneralEndpoint::from_entity_resolution_switchboard(&switchboard)?;
    let decision = switchboard
        .entity_resolution_decision()
        .map(|_| {
            ConfiguredDecisionEndpoint::from_entity_resolution_decision_switchboard(&switchboard)
                .map(|endpoint| Arc::new(endpoint) as Arc<dyn DecisionEndpoint>)
        })
        .transpose()?;
    let engine = EntityResolutionEngine::new(endpoint).with_decision_endpoint(decision);
    let rows = load_rows(results)?;
    let total_mentions = rows
        .iter()
        .map(|row| row.metadata.entity_mentions.len())
        .sum::<usize>();
    let mut rel = Cva::open(rel)?;
    let mut phy = Phylactery::open(phy)?;
    let starting_states = rel.entity_resolutions().len() + phy.entity_resolutions().len();
    let starting_unseen = total_mentions.saturating_sub(starting_states);
    println!(
        "resolution start: mode={} durable_states={starting_states}/{total_mentions} unseen={starting_unseen} rel_entities={} phy_entities={}",
        mode.label(),
        rel.entity_stats().entities,
        phy.entity_stats().entities
    );
    let mut visited = 0usize;
    let mut processed = 0usize;
    let mut new_mentions = 0usize;
    let mut reconsidered = 0usize;
    let mut unchanged_pending = 0usize;
    let mut skipped_pending = 0usize;
    let mut skipped_terminal = 0usize;
    let mut base = now_ns();
    for row in rows {
        let keys = row
            .metadata
            .entity_mentions
            .iter()
            .map(|mention| MemoryEntityMentionKey::new(row.metadata.memory_id, mention))
            .collect::<Vec<_>>();
        for key in keys {
            visited += 1;
            let existing_status = match row.owner {
                Owner::Rel => rel.entity_resolution(key).map(|value| value.status.clone()),
                Owner::Phy => phy.entity_resolution(key).map(|value| value.status.clone()),
            };
            if let Some(status) = existing_status.as_ref() {
                match status {
                    MemoryEntityResolutionStatus::Resolved { .. }
                    | MemoryEntityResolutionStatus::Rejected { .. } => {
                        skipped_terminal += 1;
                        continue;
                    }
                    MemoryEntityResolutionStatus::Pending(_)
                    | MemoryEntityResolutionStatus::Dormant(_)
                        if mode == ResolutionRunMode::UnseenOnly =>
                    {
                        skipped_pending += 1;
                        continue;
                    }
                    MemoryEntityResolutionStatus::Pending(_)
                    | MemoryEntityResolutionStatus::Dormant(_) => {}
                }
            }
            if limit.is_some_and(|limit| processed >= limit) {
                rel.sync()?;
                phy.sync()?;
                let rel_states = rel.entity_resolutions().len();
                let phy_states = phy.entity_resolutions().len();
                let durable_states = rel_states + phy_states;
                let unseen = total_mentions.saturating_sub(durable_states);
                println!(
                    "resolution batch complete: mode={} corpus={durable_states}/{total_mentions} unseen={unseen} visited={visited} work={processed} new={new_mentions} reconsidered={reconsidered} unchanged_pending={unchanged_pending} skipped_pending={skipped_pending} skipped_terminal={skipped_terminal} rel_entities={} phy_entities={} rel_states={rel_states} phy_states={phy_states}",
                    mode.label(),
                    rel.entity_stats().entities,
                    phy.entity_stats().entities
                );
                return Ok(());
            }
            base = base.saturating_add(1);
            let outcome = match row.owner {
                Owner::Rel => retry_resolution(|| {
                    rel.resolve_entity_mention_with_engine(
                        &engine,
                        key,
                        EntityCandidateConfig::default(),
                        base,
                    )
                })?,
                Owner::Phy => retry_resolution(|| {
                    phy.resolve_entity_mention_with_engine(
                        &engine,
                        key,
                        EntityCandidateConfig::default(),
                        base,
                    )
                })?,
            };
            if existing_status.is_some()
                && !outcome.resolution_changed
                && !outcome.association_changed
                && !outcome.entity_created
            {
                unchanged_pending += 1;
                continue;
            }
            processed += 1;
            if existing_status.is_some() {
                reconsidered += 1;
            } else {
                new_mentions += 1;
            }
            if processed % 50 == 0 {
                rel.sync()?;
                phy.sync()?;
                let durable_states =
                    rel.entity_resolutions().len() + phy.entity_resolutions().len();
                let unseen = total_mentions.saturating_sub(durable_states);
                println!(
                    "progress: mode={} corpus={durable_states}/{total_mentions} unseen={unseen} visited={visited} work={processed} new={new_mentions} reconsidered={reconsidered} unchanged_pending={unchanged_pending} skipped_pending={skipped_pending} skipped_terminal={skipped_terminal} rel_entities={} phy_entities={}",
                    mode.label(),
                    rel.entity_stats().entities,
                    phy.entity_stats().entities
                );
            }
        }
    }
    rel.sync()?;
    phy.sync()?;
    let rel_states = rel.entity_resolutions().len();
    let phy_states = phy.entity_resolutions().len();
    let durable_states = rel_states + phy_states;
    let unseen = total_mentions.saturating_sub(durable_states);
    println!(
        "resolution complete: mode={} corpus={durable_states}/{total_mentions} unseen={unseen} visited={visited} work={processed} new={new_mentions} reconsidered={reconsidered} unchanged_pending={unchanged_pending} skipped_pending={skipped_pending} skipped_terminal={skipped_terminal} rel_entities={} phy_entities={} rel_states={rel_states} phy_states={phy_states}",
        mode.label(),
        rel.entity_stats().entities,
        phy.entity_stats().entities
    );
    Ok(())
}

fn retry_resolution<F>(mut resolve: F) -> Result<EntityResolutionOutcome, EntityResolverError>
where
    F: FnMut() -> Result<EntityResolutionOutcome, EntityResolverError>,
{
    const MAX_RETRIES: usize = 5;
    for attempt in 0..=MAX_RETRIES {
        match resolve() {
            Ok(outcome) => return Ok(outcome),
            Err(error) if attempt < MAX_RETRIES && transient_endpoint_error(&error) => {
                let delay = retry_delay(&error, attempt);
                eprintln!(
                    "transient Entity resolution endpoint failure; retry {}/{} after {:?}: {}",
                    attempt + 1,
                    MAX_RETRIES,
                    delay,
                    error
                );
                std::thread::sleep(delay);
            }
            Err(error) => return Err(error),
        }
    }
    unreachable!("retry loop always returns")
}

fn transient_endpoint_error(error: &EntityResolverError) -> bool {
    match error {
        EntityResolverError::Endpoint(GeneralEndpointError::Backpressure {
            retry_after, ..
        }) => retry_after.is_none_or(|delay| delay <= Duration::from_secs(60)),
        EntityResolverError::Endpoint(GeneralEndpointError::Failure(message)) => {
            let lower = message.to_ascii_lowercase();
            [
                "http 500",
                "http 502",
                "http 503",
                "http 504",
                "timeout",
                "timed out",
                "connection",
                "disconnect",
                "reset",
                "temporarily unavailable",
                "request failed",
            ]
            .iter()
            .any(|needle| lower.contains(needle))
        }
        _ => false,
    }
}

fn retry_delay(error: &EntityResolverError, attempt: usize) -> Duration {
    if let EntityResolverError::Endpoint(endpoint) = error {
        if let Some(delay) = endpoint.backpressure_retry_after() {
            return delay.min(Duration::from_secs(60));
        }
    }
    Duration::from_secs(2_u64.saturating_pow(attempt as u32).min(30))
}

pub fn project_subset(
    source_rel: &str,
    source_phy: &str,
    target_rel: &str,
    target_phy: &str,
) -> Result<(), Box<dyn Error>> {
    let source_rel = Cva::open(source_rel)?;
    let source_phy = Phylactery::open(source_phy)?;
    let mut target_rel = Cva::open(target_rel)?;
    let mut target_phy = Phylactery::open(target_phy)?;
    let mut rel_entities = target_rel.entities().into_iter().map(|e| e.id).collect();
    let mut phy_entities = target_phy.entities().into_iter().map(|e| e.id).collect();
    let rel_ids = target_rel.memory_ids();
    let phy_ids = target_phy.memory_ids();
    let mut projected = 0usize;

    for id in rel_ids {
        project_rel_memory(&source_rel, &mut target_rel, id, &mut rel_entities)?;
        projected += 1;
    }
    for id in phy_ids {
        project_phy_memory(&source_phy, &mut target_phy, id, &mut phy_entities)?;
        projected += 1;
    }
    target_rel.sync()?;
    target_phy.sync()?;
    println!(
        "projection complete: memories={projected} rel_entities={} phy_entities={} rel_states={} phy_states={}",
        target_rel.entity_stats().entities,
        target_phy.entity_stats().entities,
        target_rel.entity_resolutions().len(),
        target_phy.entity_resolutions().len()
    );
    Ok(())
}

fn project_rel_memory(
    source: &Cva,
    target: &mut Cva,
    id: reliquary_memory::MemoryId,
    known: &mut HashSet<EntityId>,
) -> Result<(), Box<dyn Error>> {
    let metadata = source
        .memory_routing_metadata(id)
        .cloned()
        .ok_or("source REL missing routing metadata")?;
    if target.memory_body_id(id)? != metadata.body_id {
        return Err("target REL body mismatch".into());
    }
    target.put_memory_routing_metadata(metadata)?;
    let states = source.entity_resolutions_for_memory(id);
    let associations = source.entity_associations_for_memory(id);
    let refs = entity_refs(&states, &associations);
    copy_rel_entities(source, target, &refs, known)?;
    for state in states {
        target.import_entity_resolution(state)?;
    }
    for entity_id in associations {
        let version = target.graph_version();
        target.set_entity_association(id, entity_id, true, version)?;
    }
    Ok(())
}

fn project_phy_memory(
    source: &Phylactery,
    target: &mut Phylactery,
    id: reliquary_memory::MemoryId,
    known: &mut HashSet<EntityId>,
) -> Result<(), Box<dyn Error>> {
    let metadata = source
        .memory_routing_metadata(id)
        .cloned()
        .ok_or("source PHY missing routing metadata")?;
    if target.memory_body_id(id)? != metadata.body_id {
        return Err("target PHY body mismatch".into());
    }
    target.put_memory_routing_metadata(metadata)?;
    let states = source.entity_resolutions_for_memory(id);
    let associations = source.entity_associations_for_memory(id);
    let refs = entity_refs(&states, &associations);
    copy_phy_entities(source, target, &refs, known)?;
    for state in states {
        target.import_entity_resolution(state)?;
    }
    for entity_id in associations {
        let version = target.graph_version();
        target.set_entity_association(id, entity_id, true, version)?;
    }
    Ok(())
}

fn entity_refs(
    states: &[reliquary_memory::MemoryEntityResolution],
    associations: &[EntityId],
) -> HashSet<EntityId> {
    let mut refs = associations.iter().copied().collect::<HashSet<_>>();
    for state in states {
        match &state.status {
            MemoryEntityResolutionStatus::Resolved { entity_id, .. } => {
                refs.insert(*entity_id);
            }
            MemoryEntityResolutionStatus::Pending(value) => {
                refs.extend(value.candidate_entity_ids.iter().copied());
            }
            MemoryEntityResolutionStatus::Dormant(value) => {
                refs.extend(value.candidate_entity_ids.iter().copied());
            }
            MemoryEntityResolutionStatus::Rejected { .. } => {}
        }
    }
    refs
}

fn copy_rel_entities(
    source: &Cva,
    target: &mut Cva,
    refs: &HashSet<EntityId>,
    known: &mut HashSet<EntityId>,
) -> Result<(), Box<dyn Error>> {
    for id in refs.iter().copied() {
        if known.contains(&id) {
            continue;
        }
        let entity = source.entity(id)?;
        target.publish_entity(Some(id), 0, draft(&entity))?;
        known.insert(id);
    }
    Ok(())
}

fn copy_phy_entities(
    source: &Phylactery,
    target: &mut Phylactery,
    refs: &HashSet<EntityId>,
    known: &mut HashSet<EntityId>,
) -> Result<(), Box<dyn Error>> {
    for id in refs.iter().copied() {
        if known.contains(&id) {
            continue;
        }
        let entity = source.entity(id)?;
        target.publish_entity(Some(id), 0, draft(&entity))?;
        known.insert(id);
    }
    Ok(())
}

fn draft(entity: &reliquary_memory::Entity) -> EntityDraft {
    EntityDraft {
        canonical_name: entity.canonical_name.clone(),
        aliases: entity.aliases.clone(),
        kind: entity.kind.clone(),
        summary: entity.summary.clone(),
        mutation_id: entity.mutation_id.clone(),
        created_at_ns: entity.created_at_ns,
        updated_at_ns: entity.updated_at_ns,
    }
}

fn now_ns() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos().min(i64::MAX as u128) as i64)
        .unwrap_or(0)
}
pub fn status(rel: &str, phy: &str) -> Result<(), Box<dyn Error>> {
    let rel = Cva::open(rel)?;
    let phy = Phylactery::open(phy)?;
    print_status("rel", &rel, rel.memory_ids(), rel.entity_resolutions())?;
    print_phy_status("phy", &phy, phy.memory_ids(), phy.entity_resolutions())?;
    Ok(())
}

fn print_status(
    label: &str,
    owner: &Cva,
    ids: Vec<reliquary_memory::MemoryId>,
    states: Vec<reliquary_memory::MemoryEntityResolution>,
) -> Result<(), Box<dyn Error>> {
    let routing = ids
        .iter()
        .filter(|id| owner.memory_routing_metadata(**id).is_some())
        .count();
    let associations = ids
        .iter()
        .map(|id| owner.entity_associations_for_memory(*id).len())
        .sum::<usize>();
    let (resolved, rejected, pending, dormant) = status_counts(&states);
    println!(
        "{label}: memories={} routing={} entities={} associations={} states={} resolved={} rejected={} pending={} dormant={}",
        ids.len(),
        routing,
        owner.entity_stats().entities,
        associations,
        states.len(),
        resolved,
        rejected,
        pending,
        dormant
    );
    Ok(())
}

fn print_phy_status(
    label: &str,
    owner: &Phylactery,
    ids: Vec<reliquary_memory::MemoryId>,
    states: Vec<reliquary_memory::MemoryEntityResolution>,
) -> Result<(), Box<dyn Error>> {
    let routing = ids
        .iter()
        .filter(|id| owner.memory_routing_metadata(**id).is_some())
        .count();
    let associations = ids
        .iter()
        .map(|id| owner.entity_associations_for_memory(*id).len())
        .sum::<usize>();
    let (resolved, rejected, pending, dormant) = status_counts(&states);
    println!(
        "{label}: memories={} routing={} entities={} associations={} states={} resolved={} rejected={} pending={} dormant={}",
        ids.len(),
        routing,
        owner.entity_stats().entities,
        associations,
        states.len(),
        resolved,
        rejected,
        pending,
        dormant
    );
    Ok(())
}

fn status_counts(
    states: &[reliquary_memory::MemoryEntityResolution],
) -> (usize, usize, usize, usize) {
    let mut resolved = 0;
    let mut rejected = 0;
    let mut pending = 0;
    let mut dormant = 0;
    for state in states {
        match state.status {
            MemoryEntityResolutionStatus::Resolved { .. } => resolved += 1,
            MemoryEntityResolutionStatus::Rejected { .. } => rejected += 1,
            MemoryEntityResolutionStatus::Pending(_) => pending += 1,
            MemoryEntityResolutionStatus::Dormant(_) => dormant += 1,
        }
    }
    (resolved, rejected, pending, dormant)
}
