use crate::{CvaError, Reliquary, ReliquaryScopeKind};
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

fn test_path(name: &str) -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("reliquary-scope-{unique}"));
    fs::create_dir_all(&dir).unwrap();
    dir.join(name)
}

#[test]
fn new_reliquary_is_typed_project_rel() {
    let path = test_path("project.prj.rel");
    let rel = Reliquary::create(&path).unwrap();
    let owner_id = rel.owner_id().unwrap();
    assert!(owner_id.starts_with("proj-"));
    assert_eq!(rel.scope_kind(), ReliquaryScopeKind::Project);
    assert!(!rel.is_legacy_cva());
    drop(rel);

    let header = fs::read(&path).unwrap();
    assert_eq!(u32::from_le_bytes(header[12..16].try_into().unwrap()), 40);
    assert_eq!(header[16], 1);
    assert_eq!(header[17], 2);

    let reopened = Reliquary::open(&path).unwrap();
    assert_eq!(reopened.owner_id().as_deref(), Some(owner_id.as_str()));
    assert_eq!(reopened.scope_kind(), ReliquaryScopeKind::Project);
    assert!(!reopened.is_legacy_cva());
}

#[test]
fn reliquary_scope_kind_roundtrips() {
    let organization = test_path("acme.org.rel");
    let connection = test_path("vendor.con.rel");

    Reliquary::create_organization(&organization).unwrap();
    Reliquary::create_connection(&connection).unwrap();

    assert_eq!(
        Reliquary::open_organization(&organization)
            .unwrap()
            .scope_kind(),
        ReliquaryScopeKind::Organization
    );
    assert_eq!(
        Reliquary::open_connection(&connection)
            .unwrap()
            .scope_kind(),
        ReliquaryScopeKind::Connection
    );
    assert!(matches!(
        Reliquary::open_project(&organization),
        Err(CvaError::InvalidContainerIdentity(_))
    ));
}

#[test]
fn legacy_cva_opens_as_project_reliquary_without_rewrite() {
    let path = test_path("legacy.cva");
    let legacy = Reliquary::create_legacy_cva(&path).unwrap();
    assert!(legacy.is_legacy_cva());
    assert_eq!(legacy.scope_kind(), ReliquaryScopeKind::Project);
    drop(legacy);

    let before = fs::read(&path).unwrap();
    assert_eq!(u32::from_le_bytes(before[12..16].try_into().unwrap()), 16);

    let reopened = Reliquary::open(&path).unwrap();
    assert!(reopened.is_legacy_cva());
    assert_eq!(reopened.scope_kind(), ReliquaryScopeKind::Project);
    drop(reopened);

    let after = fs::read(&path).unwrap();
    assert_eq!(before, after);
}
