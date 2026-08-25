use crate::graph_codec::{
    GraphMutationPayload, GraphNodePayload, decode_format, decode_mutation, decode_node,
    decode_version,
};
use crate::graph_store::GraphStore;
use crate::memory_store::MemoryStore;
use crate::{ChunkRef, GraphError, GraphNodeRecord, GraphRelation};
use std::collections::{HashMap, HashSet};

pub(crate) struct GraphOpenState {
    nodes: Vec<GraphNodePayload>,
    pending: HashMap<ChunkRef, GraphMutationPayload>,
    versioned: HashSet<ChunkRef>,
    relations: Vec<GraphRelation>,
    next_graph_version: u64,
    format_seen: bool,
}

impl GraphOpenState {
    pub(crate) fn new() -> Self {
        Self {
            nodes: Vec::new(),
            pending: HashMap::new(),
            versioned: HashSet::new(),
            relations: Vec::new(),
            next_graph_version: 1,
            format_seen: false,
        }
    }

    pub(crate) fn ingest(
        &mut self,
        chunk: ChunkRef,
        payload: &[u8],
        latest_global: u64,
    ) -> Result<(), GraphError> {
        if decode_format(payload)? {
            if self.format_seen {
                return Err(GraphError::ConflictingFormat);
            }
            self.format_seen = true;
            return Ok(());
        }
        if let Some(node) = decode_node(payload)? {
            self.nodes.push(node);
            return Ok(());
        }
        if let Some(version) = decode_version(payload)? {
            self.ingest_version(chunk, version, latest_global)?;
            return Ok(());
        }
        if let Some(mutation) = decode_mutation(payload)? {
            self.pending.insert(chunk, mutation);
        }
        Ok(())
    }

    pub(crate) fn finish(self, memories: &MemoryStore) -> Result<GraphStore, GraphError> {
        if !self.format_seen {
            if self.nodes.is_empty()
                && self.pending.is_empty()
                && self.versioned.is_empty()
                && self.relations.is_empty()
            {
                return Ok(GraphStore::empty());
            }
            return Err(GraphError::MissingFormat);
        }
        let mut store = GraphStore::empty();
        store.mark_format_initialized();
        for node in self.nodes {
            store.insert_node_rebuilt(GraphNodeRecord {
                memory_id: node.memory_id,
                node_id: node.node_id,
            })?;
        }
        for relation in self.relations {
            store.insert_relation_rebuilt(relation)?;
        }
        store.finish_rebuild(memories)?;
        Ok(store)
    }

    fn ingest_version(
        &mut self,
        chunk: ChunkRef,
        version: crate::graph_codec::GraphVersionPayload,
        latest_global: u64,
    ) -> Result<(), GraphError> {
        if version.graph_version != self.next_graph_version
            || version.global_version == 0
            || version.global_version > latest_global
            || self
                .relations
                .last()
                .is_some_and(|last| last.global_version >= version.global_version)
            || version.mutation.offset >= chunk.offset
        {
            return Err(GraphError::InvalidGraphVersion);
        }
        let mutation = if let Some(mutation) = self.pending.remove(&version.mutation) {
            self.versioned.insert(version.mutation);
            mutation
        } else {
            return Err(GraphError::InvalidGraphVersion);
        };
        if mutation.source == mutation.target {
            return Err(GraphError::SelfRelation);
        }
        self.relations.push(GraphRelation {
            source: mutation.source,
            target: mutation.target,
            kind: mutation.kind,
            active: mutation.active,
            global_version: version.global_version,
            graph_version: version.graph_version,
        });
        self.next_graph_version = self
            .next_graph_version
            .checked_add(1)
            .ok_or(GraphError::GraphVersionExhausted)?;
        Ok(())
    }
}
