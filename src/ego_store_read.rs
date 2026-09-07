use super::EgoStore;
use crate::{EgoAnchor, EgoAnchorId, EgoIdentity, EgoPersonality, EgoWebSynthesis};

impl EgoStore {
    pub(crate) fn version(&self) -> u64 {
        self.version
    }

    pub(crate) fn identity(&self) -> Option<&EgoIdentity> {
        self.identity.as_ref()
    }

    pub(crate) fn personality(&self) -> Option<&EgoPersonality> {
        self.personality.as_ref()
    }

    pub(crate) fn synthesis(&self) -> Option<&EgoWebSynthesis> {
        self.synthesis.as_ref()
    }

    pub(crate) fn anchors(&self) -> Vec<EgoAnchor> {
        self.anchors
            .iter()
            .filter(|(_, state)| !state.deleted)
            .map(|(id, state)| EgoAnchor {
                id: *id,
                revision: state.revision,
                priority: state.priority,
                text: state.text.clone(),
            })
            .collect()
    }

    pub(crate) fn anchor(&self, id: EgoAnchorId) -> Option<EgoAnchor> {
        let state = self.anchors.get(&id)?;
        (!state.deleted).then(|| EgoAnchor {
            id,
            revision: state.revision,
            priority: state.priority,
            text: state.text.clone(),
        })
    }
}
