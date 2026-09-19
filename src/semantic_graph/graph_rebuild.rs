use crate::entity_store::EntityStore;
use crate::graph_codec::{
    GraphMutationPayload, GraphNodePayload, decode_batch, decode_format, decode_mutation,
    decode_node, decode_version,
};
use crate::graph_store::GraphStore;
use crate::memory_store::MemoryStore;
use crate::{GraphError, GraphNodeRecord, ObjectRef, SemanticGraphRelation};
use std::collections::{HashMap, HashSet};

pub(crate) struct GraphOpenState {
    nodes: Vec<GraphNodePayload>,
    pending: HashMap<ObjectRef, Vec<GraphMutationPayload>>,
    versioned: HashSet<ObjectRef>,
    transactions: Vec<Vec<SemanticGraphRelation>>,
    next_graph_version: u64,
    format_seen: bool,
}

impl GraphOpenState {
    pub(crate) fn new() -> Self {
        Self {
            nodes: Vec::new(),
            pending: HashMap::new(),
            versioned: HashSet::new(),
            transactions: Vec::new(),
            next_graph_version: 1,
            format_seen: false,
        }
    }

    pub(crate) fn ingest(
        &mut self,
        chunk: ObjectRef,
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
        if let Some(batch) = decode_batch(payload)? {
            self.pending.insert(chunk, batch);
            return Ok(());
        }
        if let Some(mutation) = decode_mutation(payload)? {
            self.pending.insert(chunk, vec![mutation]);
        }
        Ok(())
    }

    pub(crate) fn finish(
        self,
        memories: &MemoryStore,
        entities: &EntityStore,
    ) -> Result<GraphStore, GraphError> {
        if !self.format_seen {
            if self.nodes.is_empty()
                && self.pending.is_empty()
                && self.versioned.is_empty()
                && self.transactions.is_empty()
            {
                return Ok(GraphStore::empty());
            }
            return Err(GraphError::MissingFormat);
        }
        let mut store = GraphStore::empty();
        store.mark_format_initialized();
        for node in self.nodes {
            store.insert_node_rebuilt(GraphNodeRecord {
                semantic_node: node.semantic_node,
                node_id: node.node_id,
            })?;
        }
        for transaction in self.transactions {
            store.insert_transaction_rebuilt(&transaction)?;
        }
        store.finish_rebuild(memories, entities)?;
        Ok(store)
    }

    fn ingest_version(
        &mut self,
        chunk: ObjectRef,
        version: crate::graph_codec::GraphVersionPayload,
        latest_global: u64,
    ) -> Result<(), GraphError> {
        if version.graph_version != self.next_graph_version
            || version.global_version == 0
            || version.global_version > latest_global
            || self
                .transactions
                .last()
                .and_then(|transaction| transaction.first())
                .is_some_and(|last| last.global_version >= version.global_version)
            || !version.mutation.precedes(chunk)
        {
            return Err(GraphError::InvalidGraphVersion);
        }
        let mutations = self
            .pending
            .remove(&version.mutation)
            .ok_or(GraphError::InvalidGraphVersion)?;
        self.versioned.insert(version.mutation);

        let mut keys = HashSet::with_capacity(mutations.len());
        let mut transaction = Vec::with_capacity(mutations.len());
        for mutation in mutations {
            if mutation.source == mutation.target {
                return Err(GraphError::SelfRelation);
            }
            if !keys.insert((mutation.source, mutation.target, mutation.kind)) {
                return Err(GraphError::DuplicateRelationChange);
            }
            transaction.push(SemanticGraphRelation {
                source: mutation.source,
                target: mutation.target,
                kind: mutation.kind,
                active: mutation.active,
                origin: mutation.origin,
                global_version: version.global_version,
                graph_version: version.graph_version,
            });
        }
        self.transactions.push(transaction);
        self.next_graph_version = self
            .next_graph_version
            .checked_add(1)
            .ok_or(GraphError::GraphVersionExhausted)?;
        Ok(())
    }
}
