use crate::graph_codec::{
    GraphMutationPayload, GraphNodePayload, GraphVersionPayload, encode_batch, encode_format,
    encode_mutation, encode_node, encode_version,
};
use crate::memory_store::MemoryStore;
use crate::{
    Container, GraphError, GraphNodeRecord, GraphRelation, GraphRelationChange, GraphRelationKind,
    MemoryId,
};
use arcana::storage::InMemoryGraph;
use arcana::{GraphDataset, NodeId};
use std::collections::{HashMap, HashSet};

#[path = "graph_store_read.rs"]
mod read;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
struct RelationKey {
    source: MemoryId,
    target: MemoryId,
    kind: GraphRelationKind,
}

pub(crate) struct GraphStore {
    nodes: Vec<GraphNodeRecord>,
    node_by_memory: HashMap<MemoryId, NodeId>,
    states: HashMap<RelationKey, GraphRelation>,
    mutations: Vec<GraphRelation>,
    transaction_global_versions: Vec<u64>,
    topology: InMemoryGraph,
    next_graph_version: u64,
    format_initialized: bool,
}

impl GraphStore {
    pub(crate) fn empty() -> Self {
        Self {
            nodes: Vec::new(),
            node_by_memory: HashMap::new(),
            states: HashMap::new(),
            mutations: Vec::new(),
            transaction_global_versions: Vec::new(),
            topology: empty_topology(),
            next_graph_version: 1,
            format_initialized: false,
        }
    }

    pub(crate) fn initialize(&mut self, container: &mut Container) -> Result<(), GraphError> {
        self.ensure_format(container)
    }

    pub(crate) fn mark_format_initialized(&mut self) {
        self.format_initialized = true;
    }

    pub(crate) fn set_relation(
        &mut self,
        container: &mut Container,
        memories: &MemoryStore,
        source: MemoryId,
        target: MemoryId,
        kind: GraphRelationKind,
        active: bool,
        expected_graph_version: u64,
    ) -> Result<Option<GraphRelation>, GraphError> {
        Ok(self
            .set_relations(
                container,
                memories,
                &[GraphRelationChange {
                    source,
                    target,
                    kind,
                    active,
                }],
                expected_graph_version,
            )?
            .into_iter()
            .next())
    }

    pub(crate) fn set_relations(
        &mut self,
        container: &mut Container,
        memories: &MemoryStore,
        changes: &[GraphRelationChange],
        expected_graph_version: u64,
    ) -> Result<Vec<GraphRelation>, GraphError> {
        let current_version = self.graph_version();
        if expected_graph_version != current_version {
            return Err(GraphError::RevisionConflict {
                expected: expected_graph_version,
                actual: current_version,
            });
        }
        let mut keys = HashSet::with_capacity(changes.len());
        let mut effective = Vec::new();
        for change in changes {
            self.validate_endpoints(memories, change.source, change.target)?;
            let key = RelationKey {
                source: change.source,
                target: change.target,
                kind: change.kind,
            };
            if !keys.insert(key) {
                return Err(GraphError::DuplicateRelationChange);
            }
            let current = self
                .states
                .get(&key)
                .map(|state| state.active)
                .unwrap_or(false);
            if current != change.active {
                effective.push(*change);
            }
        }
        if effective.is_empty() {
            return Ok(Vec::new());
        }
        effective.sort_by_key(|change| (change.source.0, change.target.0, change.kind.code()));

        self.ensure_format(container)?;
        for change in &effective {
            self.ensure_node(container, change.source)?;
            self.ensure_node(container, change.target)?;
        }
        let payloads: Vec<_> = effective
            .iter()
            .map(|change| GraphMutationPayload {
                source: change.source,
                target: change.target,
                kind: change.kind,
                active: change.active,
            })
            .collect();
        let payload = if payloads.len() == 1 {
            encode_mutation(payloads[0]).to_vec()
        } else {
            encode_batch(&payloads)
        };
        let mutation = container.append(&payload)?;
        let graph_version = self.next_graph_version;
        let next_graph_version = graph_version
            .checked_add(1)
            .ok_or(GraphError::GraphVersionExhausted)?;
        let global_version = container.allocate_version()?;
        container.append(&encode_version(GraphVersionPayload {
            global_version,
            graph_version,
            mutation,
        }))?;
        let relations: Vec<_> = effective
            .into_iter()
            .map(|change| GraphRelation {
                source: change.source,
                target: change.target,
                kind: change.kind,
                active: change.active,
                global_version,
                graph_version,
            })
            .collect();
        self.insert_transaction(&relations)?;
        self.next_graph_version = next_graph_version;
        self.rebuild_topology()?;
        Ok(relations)
    }

    pub(crate) fn insert_node_rebuilt(&mut self, node: GraphNodeRecord) -> Result<(), GraphError> {
        let expected = u32::try_from(self.nodes.len()).map_err(|_| GraphError::NodeIdExhausted)?;
        if node.node_id != NodeId(expected) || self.node_by_memory.contains_key(&node.memory_id) {
            return Err(GraphError::InvalidNodeMapping);
        }
        self.node_by_memory.insert(node.memory_id, node.node_id);
        self.nodes.push(node);
        Ok(())
    }

    pub(crate) fn insert_transaction_rebuilt(
        &mut self,
        relations: &[GraphRelation],
    ) -> Result<(), GraphError> {
        if relations.is_empty()
            || relations
                .iter()
                .any(|relation| relation.graph_version != self.next_graph_version)
        {
            return Err(GraphError::InvalidGraphVersion);
        }
        self.insert_transaction(relations)?;
        self.next_graph_version = self
            .next_graph_version
            .checked_add(1)
            .ok_or(GraphError::GraphVersionExhausted)?;
        Ok(())
    }

    pub(crate) fn finish_rebuild(&mut self, memories: &MemoryStore) -> Result<(), GraphError> {
        for node in &self.nodes {
            if !memories.contains_memory(node.memory_id) {
                return Err(GraphError::MissingMemory(node.memory_id));
            }
        }
        for relation in &self.mutations {
            self.validate_endpoints(memories, relation.source, relation.target)?;
            if !self.node_by_memory.contains_key(&relation.source)
                || !self.node_by_memory.contains_key(&relation.target)
            {
                return Err(GraphError::InvalidNodeMapping);
            }
        }
        self.rebuild_topology()
    }

    fn validate_endpoints(
        &self,
        memories: &MemoryStore,
        source: MemoryId,
        target: MemoryId,
    ) -> Result<(), GraphError> {
        if source == target {
            return Err(GraphError::SelfRelation);
        }
        for memory_id in [source, target] {
            if !memories.contains_memory(memory_id) {
                return Err(GraphError::MissingMemory(memory_id));
            }
        }
        Ok(())
    }

    fn ensure_format(&mut self, container: &mut Container) -> Result<(), GraphError> {
        if !self.format_initialized {
            container.append(&encode_format())?;
            self.format_initialized = true;
        }
        Ok(())
    }

    fn ensure_node(
        &mut self,
        container: &mut Container,
        memory_id: MemoryId,
    ) -> Result<NodeId, GraphError> {
        if let Some(node_id) = self.node_by_memory.get(&memory_id).copied() {
            return Ok(node_id);
        }
        let raw = u32::try_from(self.nodes.len()).map_err(|_| GraphError::NodeIdExhausted)?;
        let node_id = NodeId(raw);
        container.append(&encode_node(GraphNodePayload { memory_id, node_id }))?;
        self.node_by_memory.insert(memory_id, node_id);
        self.nodes.push(GraphNodeRecord { memory_id, node_id });
        Ok(node_id)
    }

    fn insert_transaction(&mut self, relations: &[GraphRelation]) -> Result<(), GraphError> {
        let first = relations.first().ok_or(GraphError::InvalidGraphVersion)?;
        if relations.iter().any(|relation| {
            relation.global_version != first.global_version
                || relation.graph_version != first.graph_version
        }) || self
            .transaction_global_versions
            .last()
            .is_some_and(|previous| *previous >= first.global_version)
        {
            return Err(GraphError::InvalidGraphVersion);
        }
        let mut keys = HashSet::with_capacity(relations.len());
        for relation in relations {
            let key = RelationKey {
                source: relation.source,
                target: relation.target,
                kind: relation.kind,
            };
            if !keys.insert(key) {
                return Err(GraphError::DuplicateRelationChange);
            }
        }
        for relation in relations {
            let key = RelationKey {
                source: relation.source,
                target: relation.target,
                kind: relation.kind,
            };
            self.states.insert(key, *relation);
            self.mutations.push(*relation);
        }
        self.transaction_global_versions.push(first.global_version);
        Ok(())
    }
}

fn empty_topology() -> InMemoryGraph {
    InMemoryGraph::new(&GraphDataset {
        node_count: 0,
        edges: Vec::new(),
    })
    .expect("empty topology is valid")
}
