use crate::{
    Cva, EntityRef, Phylactery, Relationship, RelationshipDraft, RelationshipError, RelationshipId,
    RelationshipStats,
};

macro_rules! impl_owner {
    ($owner:ty) => {
        impl $owner {
            pub fn publish_relationship(
                &mut self,
                id: Option<RelationshipId>,
                expected_revision: u64,
                draft: RelationshipDraft,
            ) -> Result<(Relationship, bool), RelationshipError> {
                let owner_id = self
                    .owner_id()
                    .ok_or(RelationshipError::MissingOwnerIdentity)?;
                self.relationships.publish(
                    &mut self.container,
                    &owner_id,
                    &self.memories,
                    &self.entities,
                    id,
                    expected_revision,
                    draft,
                )
            }

            pub(crate) fn replay_relationship_revision(
                &mut self,
                id: Option<RelationshipId>,
                expected_revision: u64,
                draft: RelationshipDraft,
            ) -> Result<(Relationship, bool), RelationshipError> {
                self.relationships.publish_replayed(
                    &mut self.container,
                    id,
                    expected_revision,
                    draft,
                )
            }

            pub fn relationship(
                &self,
                id: RelationshipId,
            ) -> Result<Relationship, RelationshipError> {
                self.relationships.relationship(id)
            }

            pub fn relationships(&self) -> Vec<Relationship> {
                self.relationships.current()
            }

            pub fn relationships_for_entity(&self, entity: &EntityRef) -> Vec<Relationship> {
                self.relationships.for_entity(entity)
            }

            pub fn relationship_version(&self) -> u64 {
                self.relationships.relationship_version()
            }

            pub fn relationship_stats(&self) -> RelationshipStats {
                self.relationships.stats()
            }
        }
    };
}

impl_owner!(Cva);
impl_owner!(Phylactery);
