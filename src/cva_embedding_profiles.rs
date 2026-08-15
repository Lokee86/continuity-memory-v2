use crate::embedding_profile_probe::{profile_from_endpoint, verify_endpoint};
use crate::{
    Cva, EmbeddingEndpoint, EmbeddingProfile, EmbeddingProfileError, EmbeddingProfileId,
    EmbeddingProfileStats,
};

impl Cva {
    pub fn create_embedding_profile(
        &mut self,
        endpoint: &impl EmbeddingEndpoint,
    ) -> Result<EmbeddingProfile, EmbeddingProfileError> {
        let profile = profile_from_endpoint(endpoint)?;
        self.embedding_profiles
            .put(&mut self.container, profile.clone())?;
        Ok(profile)
    }

    pub fn embedding_profile(
        &self,
        id: EmbeddingProfileId,
    ) -> Result<EmbeddingProfile, EmbeddingProfileError> {
        self.embedding_profiles
            .get(id)
            .cloned()
            .ok_or(EmbeddingProfileError::MissingProfile)
    }

    pub fn embedding_profiles(&self) -> Vec<EmbeddingProfile> {
        self.embedding_profiles.profiles()
    }

    pub fn embedding_profile_stats(&self) -> EmbeddingProfileStats {
        self.embedding_profiles.stats()
    }

    pub fn verify_embedding_endpoint(
        &self,
        id: EmbeddingProfileId,
        endpoint: &impl EmbeddingEndpoint,
    ) -> Result<(), EmbeddingProfileError> {
        let profile = self
            .embedding_profiles
            .get(id)
            .ok_or(EmbeddingProfileError::MissingProfile)?;
        verify_endpoint(profile, endpoint)
    }
}
