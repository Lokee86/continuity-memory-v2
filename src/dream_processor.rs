use crate::{
    CompatibilityProfileId, Cva, DreamCandidateConfig, DreamClassificationError, DreamClassifier,
    DreamMemoryContext, DreamPairClassification, DreamPairVerification, DreamProcessError,
    DreamProcessResult, DreamProcessedPair, DreamVerificationError, DreamVerificationPolicy,
    DreamVerifier, GeneralEndpoint, MemoryId,
};
use std::thread;

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
        self.process_memory_with_pair_concurrency(
            cva,
            compatibility_profile_id,
            source_id,
            candidate_config,
            verification_policy,
            1,
        )
    }

    pub fn process_memory_with_pair_concurrency(
        &self,
        cva: &mut Cva,
        compatibility_profile_id: CompatibilityProfileId,
        source_id: MemoryId,
        candidate_config: DreamCandidateConfig,
        verification_policy: DreamVerificationPolicy,
        pair_concurrency: usize,
    ) -> Result<DreamProcessResult, DreamProcessError> {
        let candidates =
            cva.dream_candidates(compatibility_profile_id, source_id, candidate_config)?;
        let candidate_count = candidates.candidates.len();
        let concurrency = pair_concurrency.max(1);
        let mut evaluated = Vec::with_capacity(candidate_count);

        for chunk in candidates.candidates.chunks(concurrency) {
            let chunk_results = thread::scope(|scope| {
                let handles: Vec<_> = chunk
                    .iter()
                    .map(|candidate| {
                        scope.spawn(|| {
                            self.evaluate_pair(
                                verification_policy,
                                &candidates.source,
                                &candidate.context,
                            )
                        })
                    })
                    .collect();
                handles
                    .into_iter()
                    .map(|handle| match handle.join() {
                        Ok(result) => result,
                        Err(payload) => std::panic::resume_unwind(payload),
                    })
                    .collect::<Result<Vec<_>, DreamProcessError>>()
            })?;
            evaluated.extend(chunk_results);
        }

        let mut pairs = Vec::with_capacity(candidate_count);
        for (classification, verification) in evaluated {
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

    fn evaluate_pair(
        &self,
        verification_policy: DreamVerificationPolicy,
        source: &DreamMemoryContext,
        candidate: &DreamMemoryContext,
    ) -> Result<(DreamPairClassification, Option<DreamPairVerification>), DreamProcessError> {
        let classification = self
            .classifier
            .classify_pair(source, candidate)
            .map_err(DreamClassificationError::from)?;
        let verification = self
            .verifier
            .verify_if_required(verification_policy, &classification, source, candidate)
            .map_err(DreamVerificationError::from)?;
        Ok((classification, verification))
    }
}
