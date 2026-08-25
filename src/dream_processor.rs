use crate::{
    CompatibilityProfileId, Cva, DreamCandidateConfig, DreamClassifier, DreamProcessError,
    DreamProcessResult, DreamProcessedPair, DreamVerificationPolicy, DreamVerifier,
    GeneralEndpoint, MemoryId,
};

pub struct DreamProcessor<C, V> {
    classifier: DreamClassifier<C>,
    verifier: DreamVerifier<V>,
}

impl<C: GeneralEndpoint, V: GeneralEndpoint> DreamProcessor<C, V> {
    pub fn new(classifier_endpoint: C, verifier_endpoint: V) -> Self {
        Self {
            classifier: DreamClassifier::new(classifier_endpoint),
            verifier: DreamVerifier::new(verifier_endpoint),
        }
    }

    pub fn classifier_model(&self) -> &str {
        self.classifier.model()
    }

    pub fn verifier_model(&self) -> &str {
        self.verifier.model()
    }

    pub fn process_memory(
        &self,
        cva: &mut Cva,
        compatibility_profile_id: CompatibilityProfileId,
        source_id: MemoryId,
        candidate_config: DreamCandidateConfig,
        verification_policy: DreamVerificationPolicy,
    ) -> Result<DreamProcessResult, DreamProcessError> {
        let candidates =
            cva.dream_candidates(compatibility_profile_id, source_id, candidate_config)?;
        let candidate_count = candidates.candidates.len();
        let mut pairs = Vec::with_capacity(candidate_count);

        for candidate in &candidates.candidates {
            let classification = self
                .classifier
                .classify_pair(&candidates.source, &candidate.context)?;
            let verification = self.verifier.verify_if_required(
                verification_policy,
                &classification,
                &candidates.source,
                &candidate.context,
            )?;
            let publication = cva.publish_dream_pair(
                &classification,
                verification.as_ref(),
                verification_policy,
                cva.graph_version(),
            )?;
            pairs.push(DreamProcessedPair {
                classification,
                verification,
                publication,
            });
        }

        let lifecycle = cva.reconcile_dream_lifecycle(source_id)?;
        Ok(DreamProcessResult {
            source: lifecycle.source.clone(),
            candidate_count,
            pairs,
            lifecycle,
        })
    }
}
