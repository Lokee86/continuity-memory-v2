use super::{RelationshipStore, resolve};
use crate::relationship_model::RelationshipRecord;
use crate::{EntityRef, Relationship, RelationshipError, RelationshipId, RelationshipStats};

impl RelationshipStore {
    pub(crate) fn relationship(
        &self,
        id: RelationshipId,
    ) -> Result<Relationship, RelationshipError> {
        self.current
            .get(&id)
            .map(|index| resolve(&self.records[*index]))
            .ok_or(RelationshipError::MissingRelationship)
    }

    pub(crate) fn current(&self) -> Vec<Relationship> {
        let mut values: Vec<_> = self
            .current
            .values()
            .map(|index| resolve(&self.records[*index]))
            .collect();
        values.sort_by_key(|value| value.id);
        values
    }

    pub(crate) fn for_entity(&self, entity: &EntityRef) -> Vec<Relationship> {
        self.by_participant
            .get(entity)
            .into_iter()
            .flat_map(|ids| ids.iter())
            .filter_map(|id| self.current.get(id))
            .map(|index| resolve(&self.records[*index]))
            .collect()
    }

    pub(crate) fn records(&self) -> &[RelationshipRecord] {
        &self.records
    }

    pub(crate) fn relationship_version(&self) -> u64 {
        self.next_relationship_version.saturating_sub(1)
    }

    pub(crate) fn stats(&self) -> RelationshipStats {
        RelationshipStats {
            relationships: self.current.len(),
            revisions: self.records.len(),
            relationship_version: self.relationship_version(),
        }
    }
}
