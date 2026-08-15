use crate::embedding_profile_codec::{encode_format, encode_profile};
use crate::embedding_profile_probe::embedding_profile_id;
use crate::{
    Container, EmbeddingProfile, EmbeddingProfileError, EmbeddingProfileId, EmbeddingProfileStats,
};
use std::collections::HashMap;

#[derive(Default)]
pub(crate) struct EmbeddingProfileStore {
    profiles: HashMap<EmbeddingProfileId, EmbeddingProfile>,
}

impl EmbeddingProfileStore {
    pub(crate) fn initialize(
        &self,
        container: &mut Container,
    ) -> Result<(), EmbeddingProfileError> {
        container.append(&encode_format())?;
        Ok(())
    }

    pub(crate) fn put(
        &mut self,
        container: &mut Container,
        profile: EmbeddingProfile,
    ) -> Result<EmbeddingProfileId, EmbeddingProfileError> {
        if profile.id != embedding_profile_id(&profile) {
            return Err(EmbeddingProfileError::InvalidProfile("profile identity"));
        }
        if let Some(existing) = self.profiles.get(&profile.id) {
            return if existing == &profile {
                Ok(profile.id)
            } else {
                Err(EmbeddingProfileError::HashCollision)
            };
        }
        container.append(&encode_profile(&profile)?)?;
        let id = profile.id;
        self.profiles.insert(id, profile);
        Ok(id)
    }

    pub(crate) fn insert_rebuilt(
        &mut self,
        profile: EmbeddingProfile,
    ) -> Result<(), EmbeddingProfileError> {
        if profile.id != embedding_profile_id(&profile) {
            return Err(EmbeddingProfileError::HashCollision);
        }
        if let Some(existing) = self.profiles.get(&profile.id) {
            return if existing == &profile {
                Ok(())
            } else {
                Err(EmbeddingProfileError::HashCollision)
            };
        }
        self.profiles.insert(profile.id, profile);
        Ok(())
    }

    pub(crate) fn get(&self, id: EmbeddingProfileId) -> Option<&EmbeddingProfile> {
        self.profiles.get(&id)
    }

    pub(crate) fn profiles(&self) -> Vec<EmbeddingProfile> {
        let mut profiles: Vec<_> = self.profiles.values().cloned().collect();
        profiles.sort_by_key(|profile| profile.id.0);
        profiles
    }

    pub(crate) fn stats(&self) -> EmbeddingProfileStats {
        EmbeddingProfileStats {
            profiles: self.profiles.len(),
        }
    }
}
