use super::GraphStore;
use crate::{GraphError, GraphRelation, GraphStats, MemoryId};
use arcana_graph::storage::InMemoryGraph;
use arcana_graph::{Edge, GraphDataset, NodeId};

impl GraphStore {
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

    pub(crate) fn transaction_global_versions(&self) -> &[u64] {
        &self.transaction_global_versions
    }

    pub(crate) fn node_memory_ids(&self) -> Vec<MemoryId> {
        self.nodes.iter().map(|node| node.memory_id).collect()
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

    pub(super) fn rebuild_topology(&mut self) -> Result<(), GraphError> {
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
