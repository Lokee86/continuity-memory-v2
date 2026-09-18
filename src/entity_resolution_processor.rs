use crate::entity_resolution_evidence::{hydrate_admission_context, hydrate_evidence};
use crate::entity_resolution_processor_support::{
    bootstrap_entity_draft, candidate_fingerprint, context_fingerprint, status_reason,
    terminal_outcome,
};
use crate::{
    Cva, EntityAdmissionDecision, EntityCandidateConfig, EntityResolutionDecision,
    EntityResolutionOutcome, EntityResolver, EntityResolverError, EntityResolverOutput,
    GeneralEndpoint, MemoryEntityMentionKey, Phylactery,
};

macro_rules! impl_owner {
    ($owner:ty) => {
        impl $owner {
            pub fn resolve_entity_mention<E: GeneralEndpoint>(
                &mut self,
                resolver: &EntityResolver<E>,
                key: MemoryEntityMentionKey,
                config: EntityCandidateConfig,
                now_ns: i64,
            ) -> Result<EntityResolutionOutcome, EntityResolverError> {
                if let Some(outcome) = terminal_outcome(key, self.entity_resolution(key)) {
                    return Ok(outcome);
                }

                let set = self.entity_candidates_for_mention(key, config)?;
                let memory = self.memory(key.memory_id)?;
                let evidence = hydrate_evidence(self, &set, key.memory_id)?;
                let admission_context = if set.candidates.is_empty() {
                    hydrate_admission_context(self, &set, key.memory_id)?
                } else {
                    Vec::new()
                };
                let candidate_fingerprint = candidate_fingerprint(&set);
                let context_fingerprint =
                    context_fingerprint(&memory, &set, &evidence, &admission_context);

                if !self.entity_resolution_retry_needed(
                    key,
                    candidate_fingerprint,
                    context_fingerprint,
                ) {
                    if let Some(value) = self.entity_resolution(key) {
                        return Ok(EntityResolutionOutcome {
                            key,
                            decision: EntityResolutionDecision::Unresolved,
                            reason: status_reason(&value.status),
                            entity_id: None,
                            entity_created: false,
                            association_changed: false,
                            resolution_changed: false,
                        });
                    }
                }

                let (output, admission_materialization) = if set.candidates.is_empty() {
                    let admission = resolver.admit(&memory, &set, &admission_context)?;
                    (
                        EntityResolverOutput {
                            decision: match admission.decision {
                                EntityAdmissionDecision::CreateNew => {
                                    EntityResolutionDecision::CreateNew
                                }
                                EntityAdmissionDecision::Unresolved => {
                                    EntityResolutionDecision::Unresolved
                                }
                                EntityAdmissionDecision::Reject => EntityResolutionDecision::Reject,
                            },
                            reason: admission.reason,
                        },
                        admission.materialization,
                    )
                } else {
                    (resolver.resolve(&memory, &set, &evidence)?, None)
                };
                let expected_revision = self
                    .entity_resolution(key)
                    .map_or(0, |value| value.revision);
                match output.decision {
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
                        Ok(EntityResolutionOutcome {
                            key,
                            decision: output.decision,
                            reason: output.reason,
                            entity_id: Some(entity_id),
                            entity_created: false,
                            association_changed,
                            resolution_changed,
                        })
                    }
                    EntityResolutionDecision::CreateNew => {
                        let materialization = match admission_materialization {
                            Some(value) => value,
                            None => resolver.materialize(&memory, &set)?,
                        };
                        let draft = bootstrap_entity_draft(&memory, &set, materialization);
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
                        Ok(EntityResolutionOutcome {
                            key,
                            decision: output.decision,
                            reason: output.reason,
                            entity_id: Some(entity.id),
                            entity_created,
                            association_changed,
                            resolution_changed,
                        })
                    }
                    EntityResolutionDecision::Unresolved => {
                        let resolution_changed = self.put_entity_unresolved(
                            key,
                            expected_revision,
                            set.candidates.iter().map(|value| value.entity.id).collect(),
                            output.reason,
                            candidate_fingerprint,
                            context_fingerprint,
                            now_ns,
                        )?;
                        Ok(EntityResolutionOutcome {
                            key,
                            decision: output.decision,
                            reason: output.reason,
                            entity_id: None,
                            entity_created: false,
                            association_changed: false,
                            resolution_changed,
                        })
                    }
                    EntityResolutionDecision::Reject => {
                        let resolution_changed = self.put_entity_rejected(
                            key,
                            expected_revision,
                            output.reason,
                            now_ns,
                        )?;
                        Ok(EntityResolutionOutcome {
                            key,
                            decision: output.decision,
                            reason: output.reason,
                            entity_id: None,
                            entity_created: false,
                            association_changed: false,
                            resolution_changed,
                        })
                    }
                }
            }
        }
    };
}

impl_owner!(Cva);
impl_owner!(Phylactery);
