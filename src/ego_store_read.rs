use super::EgoStore;
use crate::{EgoAnchor, EgoAnchorId, EgoIdentity, EgoIdentityId, EgoPersonality, EgoWebSynthesis};

impl EgoStore {
    pub(crate) fn version(&self) -> u64 {
        self.version
    }

    pub(crate) fn active_identity(&self) -> Option<EgoIdentity> {
        self.active_identity_id.and_then(|id| self.identity(id))
    }

    pub(crate) fn active_identity_id(&self) -> Option<EgoIdentityId> {
        self.active_identity_id
    }

    pub(crate) fn identities(&self) -> Vec<EgoIdentity> {
        self.identities
            .iter()
            .filter(|(_, state)| !state.deleted)
            .map(|(id, state)| EgoIdentity {
                id: *id,
                revision: state.revision,
                name: state.name.clone(),
                text: state.text.clone(),
            })
            .collect()
    }

    pub(crate) fn identity(&self, id: EgoIdentityId) -> Option<EgoIdentity> {
        let state = self.identities.get(&id)?;
        (!state.deleted).then(|| EgoIdentity {
            id,
            revision: state.revision,
            name: state.name.clone(),
            text: state.text.clone(),
        })
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
