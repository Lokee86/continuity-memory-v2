use crate::{Cva, WorkspaceMetadata, WorkspaceMetadataError};
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

fn test_path() -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("continuity-workspace-metadata-{unique}"));
    fs::create_dir_all(&dir).unwrap();
    dir.join("workspace.cva")
}

fn metadata() -> WorkspaceMetadata {
    WorkspaceMetadata::new("019c-workspace-1", "Warlock Development", "software").unwrap()
}

#[test]
fn workspace_metadata_round_trips() {
    let path = test_path();
    let cva = Cva::create_workspace(&path, metadata()).unwrap();
    assert_eq!(cva.workspace_metadata(), Some(&metadata()));
    drop(cva);

    let reopened = Cva::open(path).unwrap();
    assert_eq!(reopened.workspace_metadata(), Some(&metadata()));
}

#[test]
fn ordinary_cva_can_be_initialized_as_workspace_once() {
    let path = test_path();
    let mut cva = Cva::create(&path).unwrap();
    assert_eq!(cva.workspace_metadata(), None);

    cva.initialize_workspace_metadata(metadata()).unwrap();
    cva.sync().unwrap();
    assert!(matches!(
        cva.initialize_workspace_metadata(metadata()),
        Err(WorkspaceMetadataError::AlreadyInitialized)
    ));
    drop(cva);

    let reopened = Cva::open(path).unwrap();
    assert_eq!(reopened.workspace_metadata(), Some(&metadata()));
}

#[test]
fn workspace_metadata_does_not_advance_semantic_versions() {
    let path = test_path();
    let cva = Cva::create_workspace(path, metadata()).unwrap();
    assert_eq!(cva.archive_version(), 0);
    assert_eq!(cva.memory_version(), 0);
    assert_eq!(cva.vector_generation_stats().vector_version, 0);
}

#[test]
fn workspace_metadata_rejects_blank_or_oversized_fields() {
    assert!(matches!(
        WorkspaceMetadata::new("", "name", "software"),
        Err(WorkspaceMetadataError::InvalidField("id"))
    ));
    assert!(matches!(
        WorkspaceMetadata::new("id", "   ", "software"),
        Err(WorkspaceMetadataError::InvalidField("name"))
    ));
    assert!(matches!(
        WorkspaceMetadata::new("id", "name", ""),
        Err(WorkspaceMetadataError::InvalidField("workspace_type"))
    ));
}
