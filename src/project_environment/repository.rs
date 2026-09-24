use super::ProjectRepositoryMode;
use super::git::{maintain_reserved_exclude, prepare_git_repository, require_git_repository};
use super::lore::{
    PreparedLoreRepository, current_revision, initial_checkpoint, maintain_reserved_ignore,
    prepare_managed_lore, require_lore_repository,
};
use super::lore_checkpoint;
use crate::{
    Cva, ProjectRepositoryKind, ProjectRepositoryManagement, ProjectRepositoryRef,
    ProjectRevisionCorrelation, ProjectRevisionRef,
};
use std::path::{Path, PathBuf};

pub(crate) struct PreparedProjectRepository {
    revision: ProjectRevisionRef,
    management: ProjectRepositoryManagement,
    lore: Option<PreparedLoreRepository>,
}

impl PreparedProjectRepository {
    pub(crate) fn correlate(&self, cva: &mut Cva) -> Result<(), String> {
        cva.correlate_project_revision_with_management(self.revision.clone(), self.management)
            .map(|_| ())
            .map_err(|error| error.to_string())
    }

    pub(crate) fn validate(&self, cva: &Cva) -> Result<(), String> {
        let actual = cva.latest_project_revision_correlation().ok_or_else(|| {
            "Project REL has no repository correlation after bootstrap".to_string()
        })?;
        if actual.project_revision != self.revision
            || actual.repository_management != self.management
        {
            return Err("Project REL repository correlation does not match bootstrap state".into());
        }
        Ok(())
    }

    pub(crate) fn rollback(&self) {
        if let Some(repository) = &self.lore {
            repository.rollback();
        }
    }
}

pub(crate) fn prepare_project_repository(
    project_dir: &Path,
    mode: ProjectRepositoryMode,
) -> Result<PreparedProjectRepository, String> {
    match mode {
        ProjectRepositoryMode::ManagedLore => {
            prepare_lore(project_dir, ProjectRepositoryManagement::WarlockManaged)
        }
        ProjectRepositoryMode::ManualLore => {
            prepare_lore(project_dir, ProjectRepositoryManagement::External)
        }
        ProjectRepositoryMode::Git => {
            let repository = prepare_git_repository(project_dir)?;
            Ok(PreparedProjectRepository {
                revision: ProjectRevisionRef {
                    repository: ProjectRepositoryRef {
                        kind: ProjectRepositoryKind::Git,
                        repository_id: repository.repository_id,
                        project_path: repository.project_path,
                    },
                    revision: repository.head,
                },
                management: ProjectRepositoryManagement::External,
                lore: None,
            })
        }
    }
}

fn prepare_lore(
    project_dir: &Path,
    management: ProjectRepositoryManagement,
) -> Result<PreparedProjectRepository, String> {
    let repository = prepare_managed_lore(project_dir)?;
    let revision = match initial_checkpoint(&repository, project_dir) {
        Ok(revision) => revision,
        Err(error) => {
            repository.rollback();
            return Err(error);
        }
    };
    let project_path = match repository.project_path(project_dir) {
        Ok(project_path) => project_path,
        Err(error) => {
            repository.rollback();
            return Err(error);
        }
    };
    Ok(PreparedProjectRepository {
        revision: ProjectRevisionRef {
            repository: ProjectRepositoryRef {
                kind: ProjectRepositoryKind::Lore,
                repository_id: revision.repository_id,
                project_path,
            },
            revision: revision.revision,
        },
        management,
        lore: Some(repository),
    })
}

pub(crate) fn current_revision_for(
    project_dir: &Path,
    correlation: &ProjectRevisionCorrelation,
) -> Result<ProjectRevisionRef, String> {
    let expected = &correlation.project_revision.repository;
    match expected.kind {
        ProjectRepositoryKind::Lore => {
            let repository = require_lore_repository(project_dir)?;
            let state = current_revision(&repository)?;
            if state.repository_id != expected.repository_id {
                return Err(format!(
                    "Project repository identity mismatch: REL expects {}, current repository is {}",
                    expected.repository_id, state.repository_id
                ));
            }
            maintain_reserved_ignore(&repository, project_dir)?;
            Ok(ProjectRevisionRef {
                repository: ProjectRepositoryRef {
                    kind: ProjectRepositoryKind::Lore,
                    repository_id: state.repository_id,
                    project_path: repository.project_path(project_dir)?,
                },
                revision: state.revision,
            })
        }
        ProjectRepositoryKind::Git => {
            let repository = require_git_repository(project_dir)?;
            if repository.repository_id != expected.repository_id {
                return Err(format!(
                    "Project repository identity mismatch: REL expects {}, current repository is {}",
                    expected.repository_id, repository.repository_id
                ));
            }
            maintain_reserved_exclude(&repository)?;
            Ok(ProjectRevisionRef {
                repository: ProjectRepositoryRef {
                    kind: ProjectRepositoryKind::Git,
                    repository_id: repository.repository_id,
                    project_path: repository.project_path,
                },
                revision: repository.head,
            })
        }
    }
}

pub(crate) fn repository_root(
    project_dir: &Path,
    correlation: &ProjectRevisionCorrelation,
) -> Result<PathBuf, String> {
    let expected = &correlation.project_revision.repository;
    match expected.kind {
        ProjectRepositoryKind::Lore => {
            let repository = require_lore_repository(project_dir)?;
            let current = current_revision(&repository)?;
            if current.repository_id != expected.repository_id {
                return Err("Project repository identity does not match the open REL".into());
            }
            Ok(repository.root)
        }
        ProjectRepositoryKind::Git => {
            let repository = require_git_repository(project_dir)?;
            if repository.repository_id != expected.repository_id {
                return Err("Project repository identity does not match the open REL".into());
            }
            Ok(repository.root)
        }
    }
}

pub(crate) fn checkpoint_manual_lore(
    project_dir: &Path,
    correlation: &ProjectRevisionCorrelation,
) -> Result<ProjectRevisionRef, String> {
    if correlation.project_revision.repository.kind != ProjectRepositoryKind::Lore
        || correlation.repository_management != ProjectRepositoryManagement::External
    {
        return Err(
            "explicit Lore checkpoint is available only for manually managed Lore Projects".into(),
        );
    }
    let revision = lore_checkpoint::checkpoint_project(project_dir)?;
    if revision.repository.repository_id != correlation.project_revision.repository.repository_id {
        return Err("Lore repository identity changed during checkpoint".into());
    }
    Ok(revision)
}
