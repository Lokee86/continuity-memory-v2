use crate::{Container, ContainerError};
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

fn test_path(name: &str) -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("continuity-{unique}"));
    fs::create_dir_all(&dir).unwrap();
    dir.join(name)
}

#[test]
fn create_then_reopen_legacy_container() {
    let path = test_path("archive.cva");
    let created = Container::create(&path).unwrap();
    let version = created.version();
    assert_eq!((version.major, version.minor), (1, 0));
    assert_eq!(created.identity(), None);
    drop(created);

    let mut reopened = Container::open(&path).unwrap();
    assert_eq!(reopened.chunks().unwrap().len(), 0);
    assert_eq!(reopened.identity(), None);
}

#[test]
fn typed_reliquary_identity_roundtrips() {
    let path = test_path("project.prj.rel");
    let identity = crate::ContainerIdentity {
        file_kind: crate::FileKind::Reliquary,
        scope: Some(crate::ReliquaryScopeKind::Project),
    };
    let created = Container::create_with_identity(&path, identity).unwrap();
    assert_eq!(created.identity(), Some(identity));
    drop(created);

    let reopened = Container::open(&path).unwrap();
    assert_eq!(reopened.identity(), Some(identity));
}

#[test]
fn reject_unimplemented_file_kind() {
    let path = test_path("future.phy");
    let identity = crate::ContainerIdentity {
        file_kind: crate::FileKind::Reliquary,
        scope: Some(crate::ReliquaryScopeKind::Project),
    };
    drop(Container::create_with_identity(&path, identity).unwrap());
    let mut bytes = fs::read(&path).unwrap();
    bytes[16] = 2;
    bytes[17] = 0;
    fs::write(&path, bytes).unwrap();

    assert!(matches!(
        Container::open(path),
        Err(ContainerError::InvalidIdentity)
    ));
}

#[test]
fn append_then_read_chunks() {
    let path = test_path("chunks.cva");
    let mut container = Container::create(&path).unwrap();
    let first = container.append(b"first").unwrap();
    let second = container.append(b"second record").unwrap();
    container.sync().unwrap();
    drop(container);

    let mut reopened = Container::open(&path).unwrap();
    assert_eq!(reopened.chunks().unwrap(), vec![first, second]);
    assert_eq!(reopened.read(first).unwrap(), b"first");
    assert_eq!(reopened.read(second).unwrap(), b"second record");
}

#[test]
fn reject_truncated_header() {
    let path = test_path("truncated.cva");
    fs::write(&path, b"CVA").unwrap();
    assert!(matches!(
        Container::open(path),
        Err(ContainerError::TruncatedHeader)
    ));
}

#[test]
fn recover_truncated_trailing_chunk() {
    let path = test_path("truncated-chunk.cva");
    let mut container = Container::create(&path).unwrap();
    container.append(b"complete").unwrap();
    container.sync().unwrap();
    drop(container);
    let mut bytes = fs::read(&path).unwrap();
    bytes.pop();
    fs::write(&path, bytes).unwrap();

    let mut reopened = Container::open(&path).unwrap();
    assert!(reopened.chunks().unwrap().is_empty());
    assert_eq!(fs::metadata(path).unwrap().len(), 16);
}

#[test]
fn reject_non_cva_file() {
    let path = test_path("wrong.cva");
    fs::write(&path, [0_u8; 16]).unwrap();
    assert!(matches!(
        Container::open(path),
        Err(ContainerError::InvalidMagic)
    ));
}

#[test]
fn global_versions_survive_reopen() {
    let path = test_path("versions.cva");
    let mut container = Container::create(&path).unwrap();
    assert_eq!(container.allocate_version().unwrap(), 1);
    assert_eq!(container.allocate_version().unwrap(), 2);
    container.sync().unwrap();
    drop(container);

    let mut reopened = Container::open(path).unwrap();
    assert_eq!(reopened.latest_version(), 2);
    assert_eq!(reopened.allocate_version().unwrap(), 3);
}
