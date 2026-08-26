use crate::{Cva, WorkspaceMetadata, WorkspaceMetadataError};
use std::path::Path;

impl Cva {
    pub fn create_workspace(
        path: impl AsRef<Path>,
        metadata: WorkspaceMetadata,
    ) -> Result<Self, crate::CvaError> {
        Self::create_workspace_for_scope(path, metadata, crate::ReliquaryScopeKind::Project)
    }

    pub(crate) fn create_workspace_for_scope(
        path: impl AsRef<Path>,
        metadata: WorkspaceMetadata,
        scope: crate::ReliquaryScopeKind,
    ) -> Result<Self, crate::CvaError> {
        let mut cva = Self::create_scope(path, scope)?;
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
