use crate::graph_codec::{
    GraphMutationPayload, GraphNodePayload, GraphVersionPayload, encode_format, encode_mutation,
    encode_node, encode_version,
};
use crate::memory_store::MemoryStore;
use crate::{
    Container, GraphError, GraphNodeRecord, GraphRelation, GraphRelationKind, GraphStats, MemoryId,
};
use arcana_graph::storage::InMemoryGraph;
use arcana_graph::{Edge, GraphDataset, NodeId};
use std::collections::HashMap;

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
        let current_version = self.graph_version();
        if expected_graph_version != current_version {
            return Err(GraphError::RevisionConflict {
                expected: expected_graph_version,
                actual: current_version,
            });
        }
        self.validate_endpoints(memories, source, target)?;
        let key = RelationKey {
            source,
            target,
            kind,
        };
        if self
            .states
            .get(&key)
            .is_some_and(|state| state.active == active)
            || (!active && !self.states.contains_key(&key))
        {
            return Ok(None);
        }

        self.ensure_format(container)?;
        self.ensure_node(container, source)?;
        self.ensure_node(container, target)?;
        let mutation = container.append(&encode_mutation(GraphMutationPayload {
            source,
            target,
            kind,
            active,
        }))?;
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
        let relation = GraphRelation {
            source,
            target,
            kind,
            active,
            global_version,
            graph_version,
        };
        self.insert_relation(relation)?;
        self.next_graph_version = next_graph_version;
        self.rebuild_topology()?;
        Ok(Some(relation))
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

    pub(crate) fn insert_relation_rebuilt(
        &mut self,
        relation: GraphRelation,
    ) -> Result<(), GraphError> {
        if relation.graph_version != self.next_graph_version {
            return Err(GraphError::InvalidGraphVersion);
        }
        self.insert_relation(relation)?;
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

    pub(crate) fn graph_version(&self) -> u64 {
        self.next_graph_version.saturating_sub(1)
    }

    pub(crate) fn stats(&self) -> GraphStats {
        GraphStats {
            nodes: self.nodes.len(),
            active_relations: self
                .states
                .values()
                .filter(|relation| relation.active)
                .count(),
            relation_mutations: self.mutations.len(),
            graph_version: self.graph_version(),
        }
    }

    pub(crate) fn active_relations(&self) -> Vec<GraphRelation> {
        let mut relations: Vec<_> = self
            .states
            .values()
            .filter(|relation| relation.active)
            .copied()
            .collect();
        relations
            .sort_by_key(|relation| (relation.source.0, relation.target.0, relation.kind.code()));
        relations
    }

    pub(crate) fn mutations(&self) -> &[GraphRelation] {
        &self.mutations
    }

    pub(crate) fn node_id(&self, memory_id: MemoryId) -> Result<NodeId, GraphError> {
        self.node_by_memory
            .get(&memory_id)
            .copied()
            .ok_or(GraphError::MissingNode(memory_id))
    }

    pub(crate) fn memory_id(&self, node_id: NodeId) -> Result<MemoryId, GraphError> {
        self.nodes
            .get(node_id.0 as usize)
            .map(|node| node.memory_id)
            .ok_or_else(|| GraphError::Topology(format!("unknown dense node {}", node_id.0)))
    }

    pub(crate) fn topology(&self) -> &InMemoryGraph {
        &self.topology
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

    fn insert_relation(&mut self, relation: GraphRelation) -> Result<(), GraphError> {
        let key = RelationKey {
            source: relation.source,
            target: relation.target,
            kind: relation.kind,
        };
        if let Some(previous) = self.mutations.last()
            && previous.global_version >= relation.global_version
        {
            return Err(GraphError::InvalidGraphVersion);
        }
        self.states.insert(key, relation);
        self.mutations.push(relation);
        Ok(())
    }

    fn rebuild_topology(&mut self) -> Result<(), GraphError> {
        let mut edges = Vec::new();
        for relation in self.states.values().filter(|relation| relation.active) {
            edges.push(Edge {
                source: self.node_id(relation.source)?,
                target: self.node_id(relation.target)?,
                kind: relation.kind.edge_kind(),
            });
        }
        edges.sort_unstable();
        self.topology = InMemoryGraph::new(&GraphDataset {
            node_count: u32::try_from(self.nodes.len()).map_err(|_| GraphError::NodeIdExhausted)?,
            edges,
        })
        .map_err(|error| GraphError::Topology(error.to_string()))?;
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
