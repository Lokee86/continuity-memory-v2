use super::ReliquaryRuntimeHostError;
use std::collections::{BTreeMap, BTreeSet};

pub(super) fn normalize_dependencies(mut dependencies: Vec<String>) -> Vec<String> {
    dependencies = dependencies
        .into_iter()
        .map(|value| value.trim().to_owned())
        .collect();
    dependencies.sort();
    dependencies.dedup();
    dependencies
}

pub(super) fn validate_candidate_graph(
    mut graph: BTreeMap<String, Vec<String>>,
    owner_id: &str,
    dependencies: &[String],
) -> Result<(), ReliquaryRuntimeHostError> {
    graph.insert(owner_id.to_owned(), dependencies.to_vec());
    validate_mounted_cycles(&graph)
}

pub(super) fn visit_closure(
    owner_id: &str,
    graph: &BTreeMap<String, Vec<String>>,
    visiting: &mut BTreeSet<String>,
    visited: &mut BTreeSet<String>,
    ordered: &mut Vec<String>,
) -> Result<(), ReliquaryRuntimeHostError> {
    if visited.contains(owner_id) {
        return Ok(());
    }
    if !visiting.insert(owner_id.to_owned()) {
        return Err(cycle(owner_id));
    }
    let dependencies = graph.get(owner_id).ok_or_else(|| missing(owner_id))?;
    for dependency in dependencies {
        if !graph.contains_key(dependency) {
            return Err(missing(dependency));
        }
        visit_closure(dependency, graph, visiting, visited, ordered)?;
    }
    visiting.remove(owner_id);
    visited.insert(owner_id.to_owned());
    ordered.push(owner_id.to_owned());
    Ok(())
}

fn validate_mounted_cycles(
    graph: &BTreeMap<String, Vec<String>>,
) -> Result<(), ReliquaryRuntimeHostError> {
    for owner_id in graph.keys() {
        let mut visiting = BTreeSet::new();
        let mut visited = BTreeSet::new();
        visit_mounted(owner_id, graph, &mut visiting, &mut visited)?;
    }
    Ok(())
}

fn visit_mounted(
    owner_id: &str,
    graph: &BTreeMap<String, Vec<String>>,
    visiting: &mut BTreeSet<String>,
    visited: &mut BTreeSet<String>,
) -> Result<(), ReliquaryRuntimeHostError> {
    if visited.contains(owner_id) {
        return Ok(());
    }
    if !visiting.insert(owner_id.to_owned()) {
        return Err(cycle(owner_id));
    }
    if let Some(dependencies) = graph.get(owner_id) {
        for dependency in dependencies {
            if graph.contains_key(dependency) {
                visit_mounted(dependency, graph, visiting, visited)?;
            }
        }
    }
    visiting.remove(owner_id);
    visited.insert(owner_id.to_owned());
    Ok(())
}

fn missing(owner_id: &str) -> ReliquaryRuntimeHostError {
    ReliquaryRuntimeHostError::Operation(format!("REL dependency {owner_id} is not mounted"))
}

fn cycle(owner_id: &str) -> ReliquaryRuntimeHostError {
    ReliquaryRuntimeHostError::Operation(format!("REL dependency cycle includes {owner_id}"))
}
