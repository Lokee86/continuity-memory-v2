use crate::cva_reconcile::with_replayed_transaction_time;
use crate::entity_codec::{decode_record, decode_version};
use crate::{Cva, CvaReconcileError, Entity, EntityDraft};

pub(crate) struct EntityTail {
    revisions: Vec<EntityReplayRevision>,
}

struct EntityReplayRevision {
    entity: Entity,
    transaction_time_ns: Option<i64>,
}

pub(crate) struct EntityReplayResult {
    pub(crate) revisions: usize,
    pub(crate) duplicate_revisions: usize,
}

pub(crate) fn read_entity_tail(
    cva: &mut Cva,
    start_chunk: usize,
) -> Result<EntityTail, CvaReconcileError> {
    let chunks = cva.container.chunks()?;
    let mut revisions = Vec::new();
    for chunk in chunks.iter().skip(start_chunk) {
        let payload = cva.container.read(*chunk)?;
        let Some(version) = decode_version(&payload)? else {
            continue;
        };
        let record_payload = cva.container.read(version.record)?;
        let mut record = decode_record(&record_payload)?.ok_or(
            CvaReconcileError::UnsupportedSemanticOwner("invalid Entity version record"),
        )?;
        record.global_version = version.global_version;
        record.entity_version = version.entity_version;
        revisions.push(EntityReplayRevision {
            transaction_time_ns: cva.transaction_time_ns(version.global_version),
            entity: Entity {
                id: record.id,
                revision: record.revision,
                canonical_name: record.canonical_name,
                aliases: record.aliases,
                kind: record.kind,
                summary: record.summary,
                mutation_id: record.mutation_id,
                created_at_ns: record.created_at_ns,
                updated_at_ns: record.updated_at_ns,
                global_version: record.global_version,
                entity_version: record.entity_version,
            },
        });
    }
    Ok(EntityTail { revisions })
}

pub(crate) fn replay_entity_tail(
    destination: &mut Cva,
    tail: EntityTail,
) -> Result<EntityReplayResult, CvaReconcileError> {
    let mut revisions = 0;
    let mut duplicate_revisions = 0;
    for revision in tail.revisions {
        let entity = revision.entity;
        let draft = EntityDraft {
            canonical_name: entity.canonical_name,
            aliases: entity.aliases,
            kind: entity.kind,
            summary: entity.summary,
            mutation_id: entity.mutation_id,
            created_at_ns: entity.created_at_ns,
            updated_at_ns: entity.updated_at_ns,
        };
        let (_, created) = with_replayed_transaction_time(
            destination,
            revision.transaction_time_ns,
            |destination| {
                destination.publish_entity(
                    Some(entity.id),
                    entity.revision.saturating_sub(1),
                    draft,
                )
            },
        )?;
        if created {
            revisions += 1;
        } else {
            duplicate_revisions += 1;
        }
    }
    Ok(EntityReplayResult {
        revisions,
        duplicate_revisions,
    })
}
