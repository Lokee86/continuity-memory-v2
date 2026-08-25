use crate::workspace_metadata_codec::{decode_format, decode_metadata};
use crate::workspace_metadata_store::WorkspaceMetadataStore;
use crate::{WorkspaceMetadata, WorkspaceMetadataError};

pub(crate) struct WorkspaceMetadataOpenState {
    format_seen: bool,
    metadata: Option<WorkspaceMetadata>,
}

impl WorkspaceMetadataOpenState {
    pub(crate) fn new() -> Self {
        Self {
            format_seen: false,
            metadata: None,
        }
    }

    pub(crate) fn ingest(&mut self, payload: &[u8]) -> Result<(), WorkspaceMetadataError> {
        if decode_format(payload)? {
            if self.format_seen {
                return Err(WorkspaceMetadataError::ConflictingFormat);
            }
            self.format_seen = true;
            return Ok(());
        }
        let Some(metadata) = decode_metadata(payload)? else {
            return Ok(());
        };
        if !self.format_seen {
            return Err(WorkspaceMetadataError::CorruptRecord(
                "metadata record precedes format marker",
            ));
        }
        if self.metadata.is_some() {
            return Err(WorkspaceMetadataError::CorruptRecord(
                "multiple workspace metadata records",
            ));
        }
        self.metadata = Some(metadata);
        Ok(())
    }

    pub(crate) fn finish(self) -> WorkspaceMetadataStore {
        WorkspaceMetadataStore::from_rebuild(self.format_seen, self.metadata)
    }
}
