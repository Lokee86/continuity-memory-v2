use crate::workspace_metadata_codec::{encode_format, encode_metadata};
use crate::{Container, WorkspaceMetadata, WorkspaceMetadataError};

#[derive(Debug, Default)]
pub(crate) struct WorkspaceMetadataStore {
    initialized: bool,
    metadata: Option<WorkspaceMetadata>,
}

impl WorkspaceMetadataStore {
    pub(crate) fn empty() -> Self {
        Self::default()
    }

    pub(crate) fn initialize_format(
        &mut self,
        container: &mut Container,
    ) -> Result<(), WorkspaceMetadataError> {
        if self.initialized {
            return Ok(());
        }
        container.append(&encode_format())?;
        self.initialized = true;
        Ok(())
    }

    pub(crate) fn initialize_metadata(
        &mut self,
        container: &mut Container,
        metadata: WorkspaceMetadata,
    ) -> Result<(), WorkspaceMetadataError> {
        if self.metadata.is_some() {
            return Err(WorkspaceMetadataError::AlreadyInitialized);
        }
        metadata.validate()?;
        self.initialize_format(container)?;
        container.append(&encode_metadata(&metadata)?)?;
        self.metadata = Some(metadata);
        Ok(())
    }

    pub(crate) fn metadata(&self) -> Option<&WorkspaceMetadata> {
        self.metadata.as_ref()
    }

    pub(crate) fn from_rebuild(initialized: bool, metadata: Option<WorkspaceMetadata>) -> Self {
        Self {
            initialized,
            metadata,
        }
    }
}
