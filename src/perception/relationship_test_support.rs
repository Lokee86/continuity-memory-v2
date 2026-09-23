use crate::{Cva, EntityDraft, EntityId, RelationshipDraft, RelationshipParticipant};
use std::path::PathBuf;

pub(crate) fn entity(rel: &mut Cva, name: &str, n: i64) -> EntityId {
    rel.publish_entity(
        None,
        0,
        EntityDraft {
            canonical_name: name.into(),
            aliases: Vec::new(),
            kind: "person".into(),
            summary: String::new(),
            mutation_id: format!("relationship-entity-{name}"),
            created_at_ns: n,
            updated_at_ns: n,
        },
    )
    .unwrap()
    .0
    .id
}

pub(crate) fn draft(
    participants: Vec<RelationshipParticipant>,
    mutation_id: &str,
    summary: &str,
) -> RelationshipDraft {
    RelationshipDraft {
        kind: "association".into(),
        participants,
        evidence: Vec::new(),
        summary: summary.into(),
        mutation_id: mutation_id.into(),
        created_at_ns: 1,
        updated_at_ns: 1,
    }
}

pub(crate) fn temp_path(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "reliquary-relationship-{}-{name}",
        uuid::Uuid::new_v4()
    ))
}
