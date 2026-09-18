use crate::{
    EntityAdmissionDecision, EntityResolutionDecision, EntityResolutionEvaluation,
    EntityResolutionPrepared, EntityResolver, EntityResolverError, EntityResolverOutput,
    GeneralEndpoint,
};

impl<E: GeneralEndpoint> EntityResolver<E> {
    pub(crate) fn evaluate_prepared(
        &self,
        prepared: &EntityResolutionPrepared,
    ) -> Result<EntityResolutionEvaluation, EntityResolverError> {
        if prepared.candidates.candidates.is_empty() {
            let admission = self.admit(
                &prepared.memory,
                &prepared.candidates,
                &prepared.admission_context,
            )?;
            let output = EntityResolverOutput {
                decision: match admission.decision {
                    EntityAdmissionDecision::CreateNew => EntityResolutionDecision::CreateNew,
                    EntityAdmissionDecision::Unresolved => EntityResolutionDecision::Unresolved,
                    EntityAdmissionDecision::Reject => EntityResolutionDecision::Reject,
                },
                reason: admission.reason,
            };
            return Ok(EntityResolutionEvaluation {
                output,
                materialization: admission.materialization,
            });
        }

        let output = self.resolve(&prepared.memory, &prepared.candidates, &prepared.evidence)?;
        let materialization = match output.decision {
            EntityResolutionDecision::CreateNew => {
                Some(self.materialize(&prepared.memory, &prepared.candidates)?)
            }
            _ => None,
        };
        Ok(EntityResolutionEvaluation {
            output,
            materialization,
        })
    }
}
