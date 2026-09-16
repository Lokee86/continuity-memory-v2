#[path = "ego_store_anchors.rs"]
mod anchors;
#[path = "ego_store_documents.rs"]
mod documents;
#[path = "ego_store_identities.rs"]
mod identities;
#[path = "ego_store_read.rs"]
mod read;
#[path = "ego_store_validation.rs"]
mod validation;

use crate::ego_codec::{EgoRecord, decode, encode, record_ego_version};
use crate::{
    Container, EgoAnchorId, EgoAnchorPriority, EgoError, EgoIdentityId, EgoPersonality,
    EgoWebSynthesis,
};
use std::collections::BTreeMap;

use validation::validate_record_revision;
pub(crate) use validation::validate_source_memory_version;

pub(crate) const LEGACY_IDENTITY_ID: EgoIdentityId = EgoIdentityId([0; 16]);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum EgoOwnerKind {
    Reliquary,
    Phylactery,
}

#[derive(Clone, Debug)]
struct IdentityState {
    revision: u64,
    deleted: bool,
    name: String,
    text: String,
}

#[derive(Clone, Debug)]
struct AnchorState {
    revision: u64,
    deleted: bool,
    priority: EgoAnchorPriority,
    text: String,
}

pub(crate) struct EgoStore {
    owner_kind: EgoOwnerKind,
    version: u64,
    identities: BTreeMap<EgoIdentityId, IdentityState>,
    active_identity_id: Option<EgoIdentityId>,
    personality: Option<EgoPersonality>,
    anchors: BTreeMap<EgoAnchorId, AnchorState>,
    synthesis: Option<EgoWebSynthesis>,
}

impl EgoStore {
    pub(crate) fn reliquary() -> Self {
        Self::new(EgoOwnerKind::Reliquary)
    }
    pub(crate) fn phylactery() -> Self {
        Self::new(EgoOwnerKind::Phylactery)
    }

    fn new(owner_kind: EgoOwnerKind) -> Self {
        Self {
            owner_kind,
            version: 0,
            identities: BTreeMap::new(),
            active_identity_id: None,
            personality: None,
            anchors: BTreeMap::new(),
            synthesis: None,
        }
    }

    pub(crate) fn ingest(&mut self, payload: &[u8]) -> Result<(), EgoError> {
        let Some(record) = decode(payload)? else {
            return Ok(());
        };
        self.validate_owner_record(&record)?;
        let expected_version = self.next_version()?;
        let actual_version = record_ego_version(&record);
        if actual_version != expected_version {
            return Err(EgoError::InvalidRecord(format!(
                "Ego version sequence expected {expected_version}, got {actual_version}"
            )));
        }
        self.apply(record)?;
        self.version = actual_version;
        Ok(())
    }

    fn append_apply(
        &mut self,
        container: &mut Container,
        record: EgoRecord,
    ) -> Result<(), EgoError> {
        let payload = encode(&record)?;
        container.append(&payload)?;
        let version = record_ego_version(&record);
        self.apply(record)?;
        self.version = version;
        Ok(())
    }

    fn apply(&mut self, record: EgoRecord) -> Result<(), EgoError> {
        match record {
            EgoRecord::LegacyIdentity { revision, text, .. } => {
                self.apply_identity(
                    LEGACY_IDENTITY_ID,
                    revision,
                    false,
                    true,
                    "Default".into(),
                    text,
                )?;
            }
            EgoRecord::Identity {
                id,
                revision,
                deleted,
                activate,
                name,
                text,
                ..
            } => {
                self.apply_identity(id, revision, deleted, activate, name, text)?;
            }
            EgoRecord::IdentityActive { id, .. } => {
                self.require_live_identity(id)?;
                self.active_identity_id = Some(id);
            }
            EgoRecord::Personality {
                revision,
                source_memory_version,
                text,
                ..
            } => {
                validate_record_revision(
                    self.personality.as_ref().map(|value| value.revision),
                    revision,
                )?;
                self.personality = Some(EgoPersonality {
                    revision,
                    text,
                    source_memory_version,
                });
            }
            EgoRecord::Anchor {
                id,
                revision,
                deleted,
                priority,
                text,
                ..
            } => {
                validate_record_revision(
                    self.anchors.get(&id).map(|value| value.revision),
                    revision,
                )?;
                self.anchors.insert(
                    id,
                    AnchorState {
                        revision,
                        deleted,
                        priority,
                        text,
                    },
                );
            }
            EgoRecord::Synthesis {
                revision,
                source_memory_version,
                text,
                ..
            } => {
                validate_record_revision(
                    self.synthesis.as_ref().map(|value| value.revision),
                    revision,
                )?;
                self.synthesis = Some(EgoWebSynthesis {
                    revision,
                    source_memory_version,
                    text,
                });
            }
        }
        Ok(())
    }

    fn apply_identity(
        &mut self,
        id: EgoIdentityId,
        revision: u64,
        deleted: bool,
        activate: bool,
        name: String,
        text: String,
    ) -> Result<(), EgoError> {
        validate_record_revision(
            self.identities.get(&id).map(|value| value.revision),
            revision,
        )?;
        if deleted && self.active_identity_id == Some(id) && self.live_identity_count() > 1 {
            return Err(EgoError::ActiveIdentityDeletion);
        }
        self.identities.insert(
            id,
            IdentityState {
                revision,
                deleted,
                name,
                text,
            },
        );
        if deleted && self.active_identity_id == Some(id) {
            self.active_identity_id = None;
        }
        if activate {
            self.require_live_identity(id)?;
            self.active_identity_id = Some(id);
        }
        if self.active_identity_id.is_none() && self.live_identity_count() > 0 {
            return Err(EgoError::InvalidRecord(
                "PHY contains identities but no active identity".into(),
            ));
        }
        Ok(())
    }

    fn require_live_identity(&self, id: EgoIdentityId) -> Result<(), EgoError> {
        self.identities
            .get(&id)
            .filter(|state| !state.deleted)
            .map(|_| ())
            .ok_or(EgoError::NotFound("Identity"))
    }

    fn live_identity_count(&self) -> usize {
        self.identities
            .values()
            .filter(|state| !state.deleted)
            .count()
    }

    fn validate_owner_record(&self, record: &EgoRecord) -> Result<(), EgoError> {
        if self.owner_kind == EgoOwnerKind::Reliquary
            && matches!(
                record,
                EgoRecord::LegacyIdentity { .. }
                    | EgoRecord::Identity { .. }
                    | EgoRecord::IdentityActive { .. }
                    | EgoRecord::Personality { .. }
            )
        {
            return Err(EgoError::InvalidOwnerRecord(
                "REL containers cannot contain Identity or Personality",
            ));
        }
        Ok(())
    }

    fn require_phylactery(&self, message: &'static str) -> Result<(), EgoError> {
        if self.owner_kind != EgoOwnerKind::Phylactery {
            return Err(EgoError::InvalidOwnerRecord(message));
        }
        Ok(())
    }

    fn next_version(&self) -> Result<u64, EgoError> {
        self.version
            .checked_add(1)
            .ok_or_else(|| EgoError::InvalidRecord("Ego version exhausted".into()))
    }
}
