use crate::{Cva, WorkspaceMetadata, WorkspaceMetadataError};
use std::path::Path;

impl Cva {
    pub fn create_workspace(
        path: impl AsRef<Path>,
        metadata: WorkspaceMetadata,
    ) -> Result<Self, crate::CvaError> {
        let mut cva = Self::create(path)?;
        cva.initialize_workspace_metadata(metadata)?;
        cva.sync()?;
        Ok(cva)
    }

    pub fn initialize_workspace_metadata(
        &mut self,
        metadata: WorkspaceMetadata,
    ) -> Result<(), WorkspaceMetadataError> {
        self.workspace_metadata
            .initialize_metadata(&mut self.container, metadata)
    }

    pub fn workspace_metadata(&self) -> Option<&WorkspaceMetadata> {
        self.workspace_metadata.metadata()
    }
}
