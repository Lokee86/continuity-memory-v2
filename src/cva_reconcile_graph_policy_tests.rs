use crate::cva_reconcile_graph::{GraphTail, validate_prefix_for_test};
use crate::{
    CvaReconcileConflict, CvaReconcileError, GraphRelationChange, GraphRelationKind, MemoryId,
};

fn change(active: bool) -> GraphRelationChange {
    GraphRelationChange {
        source: MemoryId([1; 32]),
        target: MemoryId([2; 32]),
        kind: GraphRelationKind::Factual,
        active,
    }
}

#[test]
fn non_prefix_graph_history_fails_closed() {
    let left = GraphTail::for_test(vec![vec![change(true)], vec![change(false)]]);
    let right = GraphTail::for_test(vec![vec![change(true)], vec![change(true)]]);

    assert!(matches!(
        validate_prefix_for_test(&left, &right),
        Err(CvaReconcileError::Conflict(
            CvaReconcileConflict::GraphRelation {
                relation_kind: GraphRelationKind::Factual,
                ..
            }
        ))
    ));
}
