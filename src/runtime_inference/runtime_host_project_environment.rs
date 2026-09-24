use super::{ReliquaryRuntimeHost, ReliquaryRuntimeHostError, operation};
use crate::{
    Cva, FileId, ProjectAdoptionSource, ProjectRepositoryMode, ProjectRevisionRef, StoredFile,
};
use std::path::{Path, PathBuf};

impl ReliquaryRuntimeHost {
    pub fn create_project_environment_rel(
        &self,
        rel_path: &Path,
        project_dir: &Path,
        mode: ProjectRepositoryMode,
    ) -> Result<Cva, ReliquaryRuntimeHostError> {
        crate::project_environment::create_project_rel(rel_path, project_dir, mode)
            .map_err(operation)
    }

    pub fn inspect_project_adoption_source(
        &self,
        rel_path: &Path,
    ) -> Result<ProjectAdoptionSource, ReliquaryRuntimeHostError> {
        crate::project_environment::inspect_project_adoption_source(rel_path).map_err(operation)
    }

    pub fn adopt_project_environment_rel(
        &self,
        source: &ProjectAdoptionSource,
        project_dir: &Path,
        mode: ProjectRepositoryMode,
    ) -> Result<Cva, ReliquaryRuntimeHostError> {
        crate::project_environment::adopt_project_rel(source, project_dir, mode).map_err(operation)
    }

    pub fn validate_project_environment_cva(
        &self,
        cva: &mut Cva,
        project_dir: &Path,
    ) -> Result<ProjectRevisionRef, ReliquaryRuntimeHostError> {
        let correlation = cva
            .latest_project_revision_correlation()
            .ok_or_else(|| operation("Project REL has no repository correlation"))?;
        let current = crate::project_environment::current_revision_for(project_dir, &correlation)
            .map_err(operation)?;
        if current.repository.project_path != correlation.project_revision.repository.project_path {
            cva.correlate_project_revision_with_management(
                current.clone(),
                correlation.repository_management,
            )
            .map_err(operation)?;
            cva.sync().map_err(operation)?;
        }
        Ok(current)
    }

    pub fn validate_project_environment_for(
        &self,
        owner_id: &str,
        project_dir: &Path,
    ) -> Result<ProjectRevisionRef, ReliquaryRuntimeHostError> {
        let correlation = self
            .latest_project_revision_correlation_for(owner_id)?
            .ok_or_else(|| operation("Project REL has no repository correlation"))?;
        let current = crate::project_environment::current_revision_for(project_dir, &correlation)
            .map_err(operation)?;
        if current.repository.project_path != correlation.project_revision.repository.project_path {
            self.correlate_project_revision_with_management_for(
                owner_id,
                current.clone(),
                correlation.repository_management,
            )?;
            self.sync_for(owner_id)?;
        }
        Ok(current)
    }

    pub fn checkpoint_manual_project_for(
        &self,
        owner_id: &str,
        project_dir: &Path,
    ) -> Result<ProjectRevisionRef, ReliquaryRuntimeHostError> {
        self.validate_project_environment_for(owner_id, project_dir)?;
        let correlation = self
            .latest_project_revision_correlation_for(owner_id)?
            .ok_or_else(|| operation("Project REL has no repository correlation"))?;
        let revision =
            crate::project_environment::checkpoint_manual_lore(project_dir, &correlation)
                .map_err(operation)?;
        self.correlate_project_revision_with_management_for(
            owner_id,
            revision.clone(),
            correlation.repository_management,
        )?;
        self.sync_for(owner_id)?;
        Ok(revision)
    }

    pub fn project_repository_root_for(
        &self,
        owner_id: &str,
        project_dir: &Path,
    ) -> Result<PathBuf, ReliquaryRuntimeHostError> {
        let correlation = self
            .latest_project_revision_correlation_for(owner_id)?
            .ok_or_else(|| operation("Project REL has no repository correlation"))?;
        crate::project_environment::repository_root(project_dir, &correlation).map_err(operation)
    }

    pub fn ingest_project_attachment(
        &self,
        project_dir: &Path,
        source: &Path,
    ) -> Result<StoredFile, ReliquaryRuntimeHostError> {
        let owner_id = self
            .active_rel_id()
            .ok_or_else(|| operation("Reliquary runtime has no active REL"))?;
        self.ingest_project_attachment_for(&owner_id, project_dir, source)
    }

    pub fn ingest_project_attachment_for(
        &self,
        owner_id: &str,
        project_dir: &Path,
        source: &Path,
    ) -> Result<StoredFile, ReliquaryRuntimeHostError> {
        let correlation = self
            .latest_project_revision_correlation_for(owner_id)?
            .ok_or_else(|| operation("Project REL has no repository correlation"))?;
        let prepared =
            crate::project_environment::prepare_upload(&correlation, project_dir, source)
                .map_err(operation)?;

        self.correlate_project_revision_with_management_for(
            owner_id,
            prepared.reference.revision.clone(),
            correlation.repository_management,
        )?;

        self.with_runtime_for(owner_id, |runtime| {
            runtime
                .cva
                .register_project_file(
                    prepared.filename,
                    prepared.mime_type,
                    prepared.byte_length,
                    prepared.reference,
                )
                .map_err(operation)
        })
    }

    pub fn resolved_file_bytes_for(
        &self,
        owner_id: &str,
        project_dir: Option<&Path>,
        file_id: FileId,
    ) -> Result<Vec<u8>, ReliquaryRuntimeHostError> {
        let reference = self.project_file_ref_for(owner_id, file_id)?;
        match reference {
            Some(reference) => {
                let project_dir = project_dir
                    .ok_or_else(|| operation("Project file has no local repository association"))?;
                crate::project_environment::read_project_file(project_dir, &reference)
                    .map_err(operation)
            }
            None => self.file_bytes_for(owner_id, file_id),
        }
    }
}
