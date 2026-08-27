use crate::{
    GraphDirection, GraphError, GraphNeighbor, GraphRelation, GraphRelationChange,
    GraphRelationKind, GraphStats, MemoryGraphPath, MemoryId, Phylactery,
};

impl Phylactery {
    pub fn set_memory_relation(
        &mut self,
        source: MemoryId,
        target: MemoryId,
        kind: GraphRelationKind,
        active: bool,
        expected_graph_version: u64,
    ) -> Result<Option<GraphRelation>, GraphError> {
        self.graph.set_relation(
            &mut self.container,
            &self.memories,
            source,
            target,
            kind,
            active,
            expected_graph_version,
        )
    }

    pub fn set_memory_relations(
        &mut self,
        changes: &[GraphRelationChange],
        expected_graph_version: u64,
    ) -> Result<Vec<GraphRelation>, GraphError> {
        self.graph.set_relations(
            &mut self.container,
            &self.memories,
            changes,
            expected_graph_version,
        )
    }

    pub fn graph_version(&self) -> u64 {
        self.graph.graph_version()
    }

    pub fn graph_stats(&self) -> GraphStats {
        self.graph.stats()
    }

    pub fn graph_relations(&self) -> Vec<GraphRelation> {
        self.graph.active_relations()
    }

    pub fn graph_neighbors(
        &self,
        memory_id: MemoryId,
        direction: GraphDirection,
    ) -> Result<Vec<GraphNeighbor>, GraphError> {
        self.graph.neighbors(memory_id, direction)
    }

    pub fn shortest_memory_path(
        &self,
        source: MemoryId,
        target: MemoryId,
        max_depth: usize,
    ) -> Result<Option<MemoryGraphPath>, GraphError> {
        self.graph.shortest_path(source, target, max_depth)
    }
}
