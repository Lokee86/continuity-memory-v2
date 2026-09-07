#[path = "ego_store_anchors.rs"]
mod anchors;
#[path = "ego_store_documents.rs"]
mod documents;
#[path = "ego_store_read.rs"]
mod read;
#[path = "ego_store_validation.rs"]
mod validation;

use crate::ego_codec::{EgoRecord, decode, encode, record_ego_version};
use crate::{
    Container, EgoAnchorId, EgoAnchorPriority, EgoError, EgoIdentity, EgoPersonality,
    EgoWebSynthesis,
};
use std::collections::BTreeMap;

use validation::validate_record_revision;
pub(crate) use validation::validate_source_memory_version;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum EgoOwnerKind {
    Reliquary,
    Phylactery,
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
    identity: Option<EgoIdentity>,
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
            identity: None,
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
            EgoRecord::Identity { revision, text, .. } => {
                validate_record_revision(
                    self.identity.as_ref().map(|value| value.revision),
                    revision,
                )?;
                self.identity = Some(EgoIdentity { revision, text });
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

    fn validate_owner_record(&self, record: &EgoRecord) -> Result<(), EgoError> {
        if self.owner_kind == EgoOwnerKind::Reliquary
            && matches!(
                record,
                EgoRecord::Identity { .. } | EgoRecord::Personality { .. }
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
