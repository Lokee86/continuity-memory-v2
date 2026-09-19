use crate::{DreamLifecycleError, GraphRelation, GraphRelationKind, MemoryId, Phylactery};
use std::collections::{HashSet, VecDeque};

pub(crate) fn canonical_superseders(
    phy: &mut Phylactery,
    source_id: MemoryId,
    relations: &[GraphRelation],
) -> Result<Vec<MemoryId>, DreamLifecycleError> {
    let mut candidates = Vec::new();
    for relation in relations.iter().filter(|relation| {
        relation.kind == GraphRelationKind::Supersedes
            && (relation.source == source_id || relation.target == source_id)
    }) {
        if unique_superseder(relations, relation.target) == Some(relation.source)
            && target_has_canonical_authority(phy, relation.target, relation.source)?
        {
            candidates.push(relation.source);
        }
    }
    candidates.sort_by_key(|id| id.0);
    candidates.dedup();
    Ok(candidates)
}

pub(crate) fn duplicate_component_has_active_peer(
    phy: &mut Phylactery,
    source_id: MemoryId,
    relations: &[GraphRelation],
) -> Result<bool, DreamLifecycleError> {
    for id in duplicate_component(source_id, relations)
        .into_iter()
        .filter(|id| *id != source_id)
    {
        if !phy.memory(id)?.archived {
            return Ok(true);
        }
    }
    Ok(false)
}

pub(crate) fn unique_superseder(relations: &[GraphRelation], target: MemoryId) -> Option<MemoryId> {
    let mut sources: Vec<_> = relations
        .iter()
        .filter(|relation| {
            relation.kind == GraphRelationKind::Supersedes && relation.target == target
        })
        .map(|relation| relation.source)
        .collect();
    sources.sort_by_key(|id| id.0);
    sources.dedup();
    match sources.as_slice() {
        [source] => Some(*source),
        _ => None,
    }
}

fn duplicate_component(source_id: MemoryId, relations: &[GraphRelation]) -> Vec<MemoryId> {
    let mut visited = HashSet::from([source_id]);
    let mut queue = VecDeque::from([source_id]);
    while let Some(current) = queue.pop_front() {
        for relation in relations.iter().filter(|relation| {
            relation.kind == GraphRelationKind::DuplicateOf
                && (relation.source == current || relation.target == current)
        }) {
            let neighbor = if relation.source == current {
                relation.target
            } else {
                relation.source
            };
            if visited.insert(neighbor) {
                queue.push_back(neighbor);
            }
        }
    }
    visited.into_iter().collect()
}

fn target_has_canonical_authority(
    phy: &mut Phylactery,
    target: MemoryId,
    superseder: MemoryId,
) -> Result<bool, DreamLifecycleError> {
    let current = phy.memory(target)?;
    if !current.archived {
        return Ok(current.lifecycle_state == "canonical");
    }
    if current.superseded_by != Some(superseder) {
        return Ok(false);
    }
    for revision in (1..current.revision).rev() {
        let previous = phy.memory_revision(target, revision)?;
        if !previous.archived {
            return Ok(previous.lifecycle_state == "canonical");
        }
    }
    Ok(false)
}
