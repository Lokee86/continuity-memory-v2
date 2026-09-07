use crate::ego_store::validate_source_memory_version;
use crate::{
    EgoAnchor, EgoAnchorId, EgoAnchorPriority, EgoIdentity, EgoPersonality, EgoWebSynthesis,
    Phylactery, PhylacteryError,
};

impl Phylactery {
    pub fn ego_version(&self) -> u64 {
        self.ego.version()
    }

    pub fn ego_identity(&self) -> Option<&EgoIdentity> {
        self.ego.identity()
    }

    pub fn put_ego_identity(
        &mut self,
        expected_revision: u64,
        text: String,
    ) -> Result<(EgoIdentity, bool), PhylacteryError> {
        Ok(self
            .ego
            .put_identity(&mut self.container, expected_revision, text)?)
    }

    pub fn ego_personality(&self) -> Option<&EgoPersonality> {
        self.ego.personality()
    }

    pub fn put_ego_personality(
        &mut self,
        expected_revision: u64,
        text: String,
        source_memory_version: u64,
    ) -> Result<(EgoPersonality, bool), PhylacteryError> {
        validate_source_memory_version(source_memory_version, self.memory_version())?;
        Ok(self.ego.put_personality(
            &mut self.container,
            expected_revision,
            text,
            source_memory_version,
        )?)
    }

    pub fn ego_anchors(&self) -> Vec<EgoAnchor> {
        self.ego.anchors()
    }

    pub fn ego_anchor(&self, id: EgoAnchorId) -> Option<EgoAnchor> {
        self.ego.anchor(id)
    }

    pub fn create_ego_anchor(
        &mut self,
        priority: EgoAnchorPriority,
        text: String,
    ) -> Result<EgoAnchor, PhylacteryError> {
        Ok(self
            .ego
            .create_anchor(&mut self.container, priority, text)?)
    }

    pub fn update_ego_anchor(
        &mut self,
        id: EgoAnchorId,
        expected_revision: u64,
        priority: EgoAnchorPriority,
        text: String,
    ) -> Result<(EgoAnchor, bool), PhylacteryError> {
        Ok(self
            .ego
            .update_anchor(&mut self.container, id, expected_revision, priority, text)?)
    }

    pub fn delete_ego_anchor(
        &mut self,
        id: EgoAnchorId,
        expected_revision: u64,
    ) -> Result<bool, PhylacteryError> {
        Ok(self
            .ego
            .delete_anchor(&mut self.container, id, expected_revision)?)
    }

    pub fn ego_web_synthesis(&self) -> Option<&EgoWebSynthesis> {
        self.ego.synthesis()
    }

    pub fn put_ego_web_synthesis(
        &mut self,
        expected_revision: u64,
        source_memory_version: u64,
        text: String,
    ) -> Result<(EgoWebSynthesis, bool), PhylacteryError> {
        validate_source_memory_version(source_memory_version, self.memory_version())?;
        Ok(self.ego.put_synthesis(
            &mut self.container,
            expected_revision,
            source_memory_version,
            text,
        )?)
    }
}
