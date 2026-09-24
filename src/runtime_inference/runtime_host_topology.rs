use super::dependency_graph::{normalize_dependencies, validate_candidate_graph, visit_closure};
use super::{
    OwnerExecution, ReliquaryRuntimeHost, ReliquaryRuntimeHostError, operation, owner_key,
};
use crate::RelMetadata;
use std::collections::{BTreeMap, BTreeSet};

impl ReliquaryRuntimeHost {
    pub(super) fn active_execution(&self) -> Result<&OwnerExecution, ReliquaryRuntimeHostError> {
        let key = self.active_key.as_ref().ok_or_else(|| {
            ReliquaryRuntimeHostError::Operation("Reliquary runtime has no active REL".into())
        })?;
        self.executions.get(key).ok_or_else(|| {
            ReliquaryRuntimeHostError::Operation("active REL execution is unavailable".into())
        })
    }

    pub(super) fn execution_for(
        &self,
        owner_id: &str,
    ) -> Result<&OwnerExecution, ReliquaryRuntimeHostError> {
        self.executions
            .get(&owner_key(Some(owner_id)))
            .ok_or_else(|| {
                ReliquaryRuntimeHostError::Operation(format!("REL owner {owner_id} is not mounted"))
            })
    }

    pub fn mounted_rel_ids(&self) -> Vec<String> {
        self.executions
            .values()
            .filter_map(|execution| execution.owner_id.clone())
            .collect()
    }

    pub fn active_rel_id(&self) -> Option<String> {
        self.active_key
            .as_ref()
            .and_then(|key| self.executions.get(key))
            .and_then(|execution| execution.owner_id.clone())
    }

    pub fn rel_metadata_for(
        &self,
        owner_id: &str,
    ) -> Result<RelMetadata, ReliquaryRuntimeHostError> {
        let runtime = self
            .execution_for(owner_id)?
            .runtime
            .as_ref()
            .cloned()
            .ok_or_else(|| {
                ReliquaryRuntimeHostError::Operation("Reliquary runtime is unavailable".into())
            })?;
        let runtime = runtime
            .lock()
            .map_err(|_| ReliquaryRuntimeHostError::LockPoisoned)?;
        Ok(runtime.cva.rel_metadata())
    }

    pub fn set_rel_metadata_for(
        &self,
        owner_id: &str,
        type_label: Option<String>,
        dependencies: Vec<String>,
    ) -> Result<bool, ReliquaryRuntimeHostError> {
        let dependencies = normalize_dependencies(dependencies);
        self.validate_candidate_topology(owner_id, &dependencies)?;
        let runtime = self
            .execution_for(owner_id)?
            .runtime
            .as_ref()
            .cloned()
            .ok_or_else(|| {
                ReliquaryRuntimeHostError::Operation("Reliquary runtime is unavailable".into())
            })?;
        let mut runtime = runtime
            .lock()
            .map_err(|_| ReliquaryRuntimeHostError::LockPoisoned)?;
        let changed = runtime
            .cva
            .set_rel_metadata(type_label, dependencies)
            .map_err(operation)?;
        if changed {
            runtime.cva.sync().map_err(operation)?;
        }
        Ok(changed)
    }

    pub fn dependency_closure(
        &self,
        owner_id: &str,
    ) -> Result<Vec<String>, ReliquaryRuntimeHostError> {
        let graph = self.mounted_dependency_graph()?;
        if !graph.contains_key(owner_id) {
            return Err(ReliquaryRuntimeHostError::Operation(format!(
                "REL owner {owner_id} is not mounted"
            )));
        }
        let mut visiting = BTreeSet::new();
        let mut visited = BTreeSet::new();
        let mut ordered = Vec::new();
        visit_closure(owner_id, &graph, &mut visiting, &mut visited, &mut ordered)?;
        Ok(ordered)
    }

    pub fn active_dependency_closure(&self) -> Result<Vec<String>, ReliquaryRuntimeHostError> {
        let owner_id = self.active_rel_id().ok_or_else(|| {
            ReliquaryRuntimeHostError::Operation(
                "active REL does not have a durable owner ID".into(),
            )
        })?;
        self.dependency_closure(&owner_id)
    }

    pub(super) fn mounted_dependency_graph(
        &self,
    ) -> Result<BTreeMap<String, Vec<String>>, ReliquaryRuntimeHostError> {
        let mut graph = BTreeMap::new();
        for execution in self.executions.values() {
            let Some(owner_id) = execution.owner_id.clone() else {
                continue;
            };
            let runtime = execution.runtime.as_ref().ok_or_else(|| {
                ReliquaryRuntimeHostError::Operation("Reliquary runtime is unavailable".into())
            })?;
            let runtime = runtime
                .lock()
                .map_err(|_| ReliquaryRuntimeHostError::LockPoisoned)?;
            graph.insert(owner_id, runtime.cva.rel_metadata().dependencies);
        }
        Ok(graph)
    }

    pub(super) fn validate_candidate_topology(
        &self,
        owner_id: &str,
        dependencies: &[String],
    ) -> Result<(), ReliquaryRuntimeHostError> {
        let graph = self.mounted_dependency_graph()?;
        validate_candidate_graph(graph, owner_id, dependencies)
    }

    pub fn rel_metadata(&self) -> Result<crate::RelMetadata, ReliquaryRuntimeHostError> {
        let runtime = self
            .active_execution()?
            .runtime
            .as_ref()
            .ok_or_else(|| {
                ReliquaryRuntimeHostError::Operation("Reliquary runtime is unavailable".into())
            })?
            .lock()
            .map_err(|_| ReliquaryRuntimeHostError::LockPoisoned)?;
        Ok(runtime.cva.rel_metadata())
    }

    pub fn set_rel_metadata(
        &self,
        type_label: Option<String>,
        dependencies: Vec<String>,
    ) -> Result<bool, ReliquaryRuntimeHostError> {
        if let Some(owner_id) = self.active_rel_id() {
            return self.set_rel_metadata_for(&owner_id, type_label, dependencies);
        }
        let mut runtime = self
            .active_execution()?
            .runtime
            .as_ref()
            .ok_or_else(|| {
                ReliquaryRuntimeHostError::Operation("Reliquary runtime is unavailable".into())
            })?
            .lock()
            .map_err(|_| ReliquaryRuntimeHostError::LockPoisoned)?;
        let changed = runtime
            .cva
            .set_rel_metadata(type_label, dependencies)
            .map_err(operation)?;
        if changed {
            runtime.cva.sync().map_err(operation)?;
        }
        Ok(changed)
    }
}
