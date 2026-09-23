use crate::entity_candidate_test_support::{
    entity_draft, install_rel_mention, publish_rel_memory, temp_path,
};
use crate::{Cva, EntityResolutionReason};

#[test]
fn reports_zero_degree_and_surface_collisions() {
    let mut rel = Cva::create_project(temp_path("entity-audit.rel")).unwrap();
    let memory = publish_rel_memory(&mut rel, "audit", "School", "Eastwood is a school.");
    let key = install_rel_mention(&mut rel, memory, "Eastwood");
    let first = rel
        .publish_entity(None, 0, entity_draft("Eastwood", &[], "first", 1))
        .unwrap()
        .0;
    let _second = rel
        .publish_entity(None, 0, entity_draft("east-wood", &[], "second", 2))
        .unwrap()
        .0;
    rel.set_entity_association(memory, first.id, true, rel.graph_version())
        .unwrap();
    rel.put_entity_resolved(key, 0, first.id, EntityResolutionReason::ContextMatch, 3)
        .unwrap();

    let report = rel.audit_entities();
    assert_eq!(report.entity_count, 2);
    assert_eq!(report.association_count, 1);
    assert_eq!(report.zero_degree_entities.len(), 1);
    assert_eq!(report.normalized_surface_collisions.len(), 1);
    assert!(!report.is_clean());
}
