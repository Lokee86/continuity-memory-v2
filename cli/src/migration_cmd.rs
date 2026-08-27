use anyhow::Result;
use reliquary_memory::{FileKind, migrate_file};
use std::path::Path;

pub fn run(source: &Path, output: &Path) -> Result<()> {
    let result = migrate_file(source, output)?;
    let kind = match result.file_kind {
        FileKind::Reliquary => "REL",
        FileKind::Phylactery => "PHY",
    };
    let identity = if result.derived_from_legacy_workspace_id {
        "legacy-workspace-id"
    } else {
        "generated"
    };
    println!(
        "migrated {kind}: {} -> {} id={} identity_source={identity}",
        source.display(),
        output.display(),
        result.owner_id
    );
    Ok(())
}
