use crate::graph_codec::{decode_batch, decode_mutation, decode_version};
use crate::{
    Cva, CvaReconcileConflict, CvaReconcileError, GraphRelationChange, GraphRelationKind, MemoryId,
};
use std::collections::HashMap;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
struct GraphKey {
    source: MemoryId,
    target: MemoryId,
    kind: GraphRelationKind,
}

pub(crate) struct GraphTail {
    transactions: Vec<Vec<GraphRelationChange>>,
}

#[cfg(test)]
impl GraphTail {
    pub(crate) fn for_test(transactions: Vec<Vec<GraphRelationChange>>) -> Self {
        Self { transactions }
    }
}

pub(crate) struct GraphReplayResult {
    pub(crate) transactions: usize,
    pub(crate) mutations: usize,
    pub(crate) duplicate_mutations: usize,
}

pub(crate) fn read_graph_tail(
    cva: &mut Cva,
    start_chunk: usize,
) -> Result<GraphTail, CvaReconcileError> {
    let chunks = cva.container.chunks()?;
    let mut transactions = Vec::new();
    for chunk in chunks.iter().skip(start_chunk) {
        let payload = cva.container.read(*chunk)?;
        let Some(version) = decode_version(&payload)? else {
            continue;
        };
        let mutation_payload = cva.container.read(version.mutation)?;
        let mutations = if let Some(batch) = decode_batch(&mutation_payload)? {
            batch
        } else if let Some(mutation) = decode_mutation(&mutation_payload)? {
            vec![mutation]
        } else {
            return Err(CvaReconcileError::InvalidGraphVersionRecord);
        };
        transactions.push(
            mutations
                .into_iter()
                .map(|mutation| GraphRelationChange {
                    source: mutation.source,
                    target: mutation.target,
                    kind: mutation.kind,
                    active: mutation.active,
                })
                .collect(),
        );
    }
    Ok(GraphTail { transactions })
}

pub(crate) fn replay_graph_tail(
    destination: &mut Cva,
    tail: &GraphTail,
) -> Result<GraphReplayResult, CvaReconcileError> {
    replay_with_skips(destination, tail, &HashMap::new())
}

pub(crate) fn reconcile_right_graph_tail(
    destination: &mut Cva,
    left_divergent: &GraphTail,
    right_divergent: &GraphTail,
) -> Result<GraphReplayResult, CvaReconcileError> {
    let skips = replay_skips(left_divergent, right_divergent)?;
    replay_with_skips(destination, right_divergent, &skips)
}

#[cfg(test)]
pub(crate) fn validate_prefix_for_test(
    left: &GraphTail,
    right: &GraphTail,
) -> Result<(), CvaReconcileError> {
    replay_skips(left, right).map(|_| ())
}

fn replay_skips(
    left: &GraphTail,
    right: &GraphTail,
) -> Result<HashMap<GraphKey, usize>, CvaReconcileError> {
    let left_sequences = sequences(left);
    let right_sequences = sequences(right);
    let mut skips = HashMap::new();
    for (key, right_states) in right_sequences {
        let Some(left_states) = left_sequences.get(&key) else {
            continue;
        };
        let common = left_states
            .iter()
            .zip(&right_states)
            .take_while(|(left, right)| left == right)
            .count();
        if common != left_states.len().min(right_states.len()) {
            return Err(CvaReconcileError::Conflict(
                CvaReconcileConflict::GraphRelation {
                    source: key.source,
                    target: key.target,
                    relation_kind: key.kind,
                    left_states: left_states.clone(),
                    right_states,
                },
            ));
        }
        if common > 0 {
            skips.insert(key, common);
        }
    }
    Ok(skips)
}

fn replay_with_skips(
    destination: &mut Cva,
    tail: &GraphTail,
    skips: &HashMap<GraphKey, usize>,
) -> Result<GraphReplayResult, CvaReconcileError> {
    let mut seen = HashMap::<GraphKey, usize>::new();
    let mut transactions = 0;
    let mut mutations = 0;
    let mut duplicate_mutations = 0;
    for transaction in &tail.transactions {
        let mut changes = Vec::with_capacity(transaction.len());
        for change in transaction {
            let key = key(*change);
            let occurrence = seen.entry(key).or_default();
            if *occurrence < skips.get(&key).copied().unwrap_or(0) {
                duplicate_mutations += 1;
            } else {
                changes.push(*change);
            }
            *occurrence += 1;
        }
        if changes.is_empty() {
            continue;
        }
        let published = destination.set_memory_relations(&changes, destination.graph_version())?;
        duplicate_mutations += changes.len().saturating_sub(published.len());
        if !published.is_empty() {
            transactions += 1;
            mutations += published.len();
        }
    }
    Ok(GraphReplayResult {
        transactions,
        mutations,
        duplicate_mutations,
    })
}

fn sequences(tail: &GraphTail) -> HashMap<GraphKey, Vec<bool>> {
    let mut sequences = HashMap::<GraphKey, Vec<bool>>::new();
    for transaction in &tail.transactions {
        for change in transaction {
            sequences
                .entry(key(*change))
                .or_default()
                .push(change.active);
        }
    }
    sequences
}

fn key(change: GraphRelationChange) -> GraphKey {
    GraphKey {
        source: change.source,
        target: change.target,
        kind: change.kind,
    }
}
