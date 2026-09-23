use crate::{Entity, EntityId};
use std::collections::HashMap;

pub(crate) fn merge_alias_budget_fits(survivor: &Entity, retired: &Entity) -> bool {
    let mut aliases = survivor
        .aliases
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>();
    for surface in std::iter::once(retired.canonical_name.as_str())
        .chain(retired.aliases.iter().map(String::as_str))
    {
        if survivor.canonical_name.eq_ignore_ascii_case(surface)
            || aliases
                .iter()
                .any(|alias| alias.eq_ignore_ascii_case(surface))
        {
            continue;
        }
        aliases.push(surface);
    }
    aliases.len() <= crate::MAX_ENTITY_ALIASES
}

pub(crate) fn choose_survivor(
    left: &Entity,
    right: &Entity,
    degrees: &HashMap<EntityId, usize>,
) -> (EntityId, EntityId) {
    let left_key = (
        std::cmp::Reverse(degrees.get(&left.id).copied().unwrap_or(0)),
        left.created_at_ns,
        std::cmp::Reverse(left.canonical_name.len()),
        left.id,
    );
    let right_key = (
        std::cmp::Reverse(degrees.get(&right.id).copied().unwrap_or(0)),
        right.created_at_ns,
        std::cmp::Reverse(right.canonical_name.len()),
        right.id,
    );
    if left_key <= right_key {
        (left.id, right.id)
    } else {
        (right.id, left.id)
    }
}

pub(crate) fn exact_entity_for_surface(entities: &[Entity], surface: &str) -> Option<Entity> {
    let mut matches = entities.iter().filter(|entity| {
        entity
            .canonical_name
            .trim()
            .eq_ignore_ascii_case(surface.trim())
            || entity
                .aliases
                .iter()
                .any(|alias| alias.trim().eq_ignore_ascii_case(surface.trim()))
    });
    let first = matches.next()?.clone();
    matches.next().is_none().then_some(first)
}
