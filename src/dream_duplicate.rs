use crate::dream_duplicate_index::{DuplicateIndex, DuplicateTemporalKey};
use crate::dream_source_time::source_timestamp_ns;
use crate::{Cva, DreamPublicationError, DreamPublicationOutcome, GraphRelationKind, MemoryId};
use std::collections::{HashMap, HashSet};

impl Cva {
    pub(crate) fn publish_duplicate_pair(
        &mut self,
        a: MemoryId,
        b: MemoryId,
        expected_graph_version: u64,
    ) -> Result<DreamPublicationOutcome, DreamPublicationError> {
        if expected_graph_version != self.graph.graph_version() {
            return Err(crate::GraphError::RevisionConflict {
                expected: expected_graph_version,
                actual: self.graph.graph_version(),
            }
            .into());
        }
        self.ensure_duplicate_index()?;
        let a_key = self.duplicate_key(a)?;
        let b_key = self.duplicate_key(b)?;
        let relations = self.graph.active_relations();
        let mut next_index = self.duplicate_index.clone();
        let mut changes = next_index.plan_union(a, a_key, b, b_key, &relations);
        for relation in relations.iter().filter(|relation| {
            same_pair(a, b, relation.source, relation.target)
                && is_nonduplicate_primary(relation.kind)
        }) {
            changes.push(crate::GraphRelationChange {
                source: relation.source,
                target: relation.target,
                kind: relation.kind,
                active: false,
            });
        }
        changes.sort_by_key(|change| {
            (
                change.source.0,
                change.target.0,
                change.kind.code(),
                change.active,
            )
        });
        if changes.is_empty() {
            return Ok(DreamPublicationOutcome::NoChange);
        }
        let published = self.graph.set_relations(
            &mut self.container,
            &self.memories,
            &changes,
            expected_graph_version,
        )?;
        next_index.set_graph_version(self.graph.graph_version());
        self.duplicate_index = next_index;
        Ok(DreamPublicationOutcome::Published(published))
    }

    fn ensure_duplicate_index(&mut self) -> Result<(), DreamPublicationError> {
        let graph_version = self.graph.graph_version();
        if self.duplicate_index.graph_version() == Some(graph_version) {
            return Ok(());
        }
        let relations = self.graph.active_relations();
        let ids: HashSet<_> = relations
            .iter()
            .filter(|relation| relation.kind == GraphRelationKind::DuplicateOf && relation.active)
            .flat_map(|relation| [relation.source, relation.target])
            .collect();
        let mut keys = HashMap::with_capacity(ids.len());
        for memory_id in ids {
            keys.insert(memory_id, self.duplicate_key(memory_id)?);
        }
        self.duplicate_index = DuplicateIndex::rebuild(graph_version, &relations, &keys);
        Ok(())
    }

    fn duplicate_key(
        &mut self,
        memory_id: MemoryId,
    ) -> Result<DuplicateTemporalKey, DreamPublicationError> {
        let memory = self.memories.memory(&mut self.container, memory_id)?;
        let timestamp_ns = source_timestamp_ns(self, &memory)
            .ok_or(DreamPublicationError::MissingSourceTimestamp(memory_id))?;
        Ok(DuplicateTemporalKey::new(timestamp_ns, memory_id))
    }
}

fn same_pair(a: MemoryId, b: MemoryId, source: MemoryId, target: MemoryId) -> bool {
    (source == a && target == b) || (source == b && target == a)
}

fn is_nonduplicate_primary(kind: GraphRelationKind) -> bool {
    matches!(
        kind,
        GraphRelationKind::Topical
            | GraphRelationKind::Factual
            | GraphRelationKind::Causal
            | GraphRelationKind::Recurrent
            | GraphRelationKind::Supersedes
    )
}
