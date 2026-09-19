use crate::compatibility_profile_probe::{profile_from_endpoint, verify_endpoint};
use crate::{
    CompatibilityProfile, CompatibilityProfileError, CompatibilityProfileId,
    CompatibilityProfileStats, CompatibilityReport, EmbeddingEndpoint, Phylactery,
};

impl Phylactery {
    pub fn establish_compatibility_profile(
        &mut self,
        endpoint: &impl EmbeddingEndpoint,
    ) -> Result<CompatibilityProfile, CompatibilityProfileError> {
        let candidate = profile_from_endpoint(endpoint)?;
        if let Some(existing) = self.compatibility_profiles.compatible_with(&candidate) {
            return Ok(existing.clone());
        }
        self.compatibility_profiles
            .put(&mut self.container, candidate.clone())?;
        Ok(candidate)
    }

    pub fn compatibility_profile(
        &self,
        id: CompatibilityProfileId,
    ) -> Result<CompatibilityProfile, CompatibilityProfileError> {
        self.compatibility_profiles
            .get(id)
            .cloned()
            .ok_or(CompatibilityProfileError::MissingProfile)
    }

    pub fn compatibility_profiles(&self) -> Vec<CompatibilityProfile> {
        self.compatibility_profiles.profiles()
    }

    pub fn compatibility_profile_stats(&self) -> CompatibilityProfileStats {
        self.compatibility_profiles.stats()
    }

    pub fn verify_compatibility_endpoint(
        &self,
        id: CompatibilityProfileId,
        endpoint: &impl EmbeddingEndpoint,
    ) -> Result<CompatibilityReport, CompatibilityProfileError> {
        let profile = self
            .compatibility_profiles
            .get(id)
            .ok_or(CompatibilityProfileError::MissingProfile)?;
        verify_endpoint(profile, endpoint)
    }
}
