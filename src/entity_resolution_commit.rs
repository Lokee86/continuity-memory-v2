use crate::entity_resolution_processor_support::bootstrap_entity_draft;
use crate::{
    Cva, EntityResolutionDecision, EntityResolutionEvaluation, EntityResolutionOutcome,
    EntityResolutionPrepared, EntityResolverError, Phylactery,
};

macro_rules! impl_owner {
    ($owner:ty) => {
        impl $owner {
            pub(crate) fn commit_entity_resolution(
                &mut self,
                prepared: EntityResolutionPrepared,
                evaluation: EntityResolutionEvaluation,
                now_ns: i64,
            ) -> Result<Option<EntityResolutionOutcome>, EntityResolverError> {
                if self.owner_id().as_deref() != Some(prepared.owner_id.as_str())
                    || self.memory_version() != prepared.memory_version
                    || self.entity_version() != prepared.entity_version
                    || self.graph_version() != prepared.graph_version
                    || self
                        .entity_resolution(prepared.key)
                        .map_or(0, |value| value.revision)
                        != prepared.expected_resolution_revision
                {
                    return Ok(None);
                }

                let key = prepared.key;
                let output = evaluation.output;
                let expected_revision = prepared.expected_resolution_revision;
                let outcome = match output.decision {
                    EntityResolutionDecision::ResolveExisting(entity_id) => {
                        let association_changed = self
                            .set_entity_association(
                                key.memory_id,
                                entity_id,
                                true,
                                self.graph_version(),
                            )?
                            .is_some();
                        let resolution_changed = self.put_entity_resolved(
                            key,
                            expected_revision,
                            entity_id,
                            output.reason,
                            now_ns,
                        )?;
                        EntityResolutionOutcome {
                            key,
                            decision: output.decision,
                            reason: output.reason,
                            entity_id: Some(entity_id),
                            entity_created: false,
                            association_changed,
                            resolution_changed,
                        }
                    }
                    EntityResolutionDecision::CreateNew => {
                        let materialization = evaluation.materialization.ok_or_else(|| {
                            EntityResolverError::InvalidOutput(
                                "create_new evaluation missing Entity metadata".into(),
                            )
                        })?;
                        let draft = bootstrap_entity_draft(
                            &prepared.memory,
                            &prepared.candidates,
                            materialization,
                        );
                        let (entity, entity_created) = self.publish_entity(None, 0, draft)?;
                        let association_changed = self
                            .set_entity_association(
                                key.memory_id,
                                entity.id,
                                true,
                                self.graph_version(),
                            )?
                            .is_some();
                        let resolution_changed = self.put_entity_resolved(
                            key,
                            expected_revision,
                            entity.id,
                            output.reason,
                            now_ns,
                        )?;
                        EntityResolutionOutcome {
                            key,
                            decision: output.decision,
                            reason: output.reason,
                            entity_id: Some(entity.id),
                            entity_created,
                            association_changed,
                            resolution_changed,
                        }
                    }
                    EntityResolutionDecision::Unresolved => {
                        let resolution_changed = self.put_entity_unresolved(
                            key,
                            expected_revision,
                            prepared
                                .candidates
                                .candidates
                                .iter()
                                .map(|value| value.entity.id)
                                .collect(),
                            output.reason,
                            prepared.candidate_fingerprint,
                            prepared.context_fingerprint,
                            now_ns,
                        )?;
                        EntityResolutionOutcome {
                            key,
                            decision: output.decision,
                            reason: output.reason,
                            entity_id: None,
                            entity_created: false,
                            association_changed: false,
                            resolution_changed,
                        }
                    }
                    EntityResolutionDecision::Reject => {
                        let resolution_changed = self.put_entity_rejected(
                            key,
                            expected_revision,
                            output.reason,
                            now_ns,
                        )?;
                        EntityResolutionOutcome {
                            key,
                            decision: output.decision,
                            reason: output.reason,
                            entity_id: None,
                            entity_created: false,
                            association_changed: false,
                            resolution_changed,
                        }
                    }
                };
                Ok(Some(outcome))
            }
        }
    };
}

impl_owner!(Cva);
impl_owner!(Phylactery);
