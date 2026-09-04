use crate::graph_store::GraphStore;
use crate::{
    GraphDirection, GraphError, GraphNeighbor, GraphRelationKind, MemoryGraphPath, MemoryId,
};
use arcana::storage::Neighbor;
use arcana::traversal::{TraversalError, shortest_path};
use arcana::{EdgeKind, NodeId};

impl GraphStore {
    pub(crate) fn neighbors(
        &self,
        memory_id: MemoryId,
        direction: GraphDirection,
    ) -> Result<Vec<GraphNeighbor>, GraphError> {
        let node_id = self.node_id(memory_id)?;
        let neighbors = match direction {
            GraphDirection::Outgoing => self.topology().forward_neighbors(node_id),
            GraphDirection::Incoming => self.topology().reverse_neighbors(node_id),
        }
        .map_err(|error| GraphError::Topology(error.to_string()))?;
        neighbors
            .iter()
            .map(|neighbor| {
                Ok(GraphNeighbor {
                    memory_id: self.memory_id(neighbor.node)?,
                    kind: relation_kind(neighbor.kind)?,
                })
            })
            .collect()
    }

    pub(crate) fn shortest_path(
        &self,
        source: MemoryId,
        target: MemoryId,
        max_depth: usize,
    ) -> Result<Option<MemoryGraphPath>, GraphError> {
        let source_node = self.node_id(source)?;
        let target_node = self.node_id(target)?;
        let path = shortest_path(
            self.topology().node_count(),
            source_node,
            target_node,
            max_depth,
            |node| topology_neighbors(self, node),
        )
        .map_err(map_traversal_error)?;
        path.map(|path| {
            let memories = path
                .nodes
                .into_iter()
                .map(|node| self.memory_id(node))
                .collect::<Result<Vec<_>, _>>()?;
            let relations = path
                .kinds
                .into_iter()
                .map(relation_kind)
                .collect::<Result<Vec<_>, _>>()?;
            Ok(MemoryGraphPath {
                memories,
                relations,
            })
        })
        .transpose()
    }
}

fn topology_neighbors(
    store: &GraphStore,
    node: NodeId,
) -> Result<Vec<Neighbor>, arcana::storage::QueryError> {
    store
        .topology()
        .forward_neighbors(node)
        .map(|values| values.to_vec())
}

fn relation_kind(kind: EdgeKind) -> Result<GraphRelationKind, GraphError> {
    GraphRelationKind::from_code(kind.0).ok_or(GraphError::UnknownRelationKind(kind.0))
}

fn map_traversal_error(error: TraversalError<arcana::storage::QueryError>) -> GraphError {
    GraphError::Topology(error.to_string())
}
