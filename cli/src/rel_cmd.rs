use crate::args::{RelCommand, RelScopeArg};
use crate::util::{file_len, hex32};
use anyhow::Result;
use reliquary_memory::{Reliquary, ReliquaryScopeKind};

pub fn run(command: RelCommand) -> Result<()> {
    match command {
        RelCommand::Create { path, scope } => {
            let rel = create(&path, scope)?;
            println!(
                "created REL: {} id={} scope={}",
                path.display(),
                rel.owner_id().as_deref().unwrap_or("none"),
                scope_name(scope)
            );
        }
        RelCommand::Info { path } => info(&path)?,
        RelCommand::Verify { path } => {
            let rel = Reliquary::open(&path)?;
            println!(
                "REL ok: {} id={} scope={} legacy_cva={} archive_version={} vector_version={}",
                path.display(),
                rel.owner_id().as_deref().unwrap_or("none"),
                scope_kind_name(rel.scope_kind()),
                rel.is_legacy_cva(),
                rel.archive_version(),
                rel.vector_version()
            );
        }
    }
    Ok(())
}

fn create(path: &std::path::Path, scope: RelScopeArg) -> Result<Reliquary> {
    Ok(match scope {
        RelScopeArg::Organization => Reliquary::create_organization(path)?,
        RelScopeArg::Project => Reliquary::create_project(path)?,
        RelScopeArg::Connection => Reliquary::create_connection(path)?,
    })
}

fn info(path: &std::path::Path) -> Result<()> {
    let rel = Reliquary::open(path)?;
    let archive = rel.stats();
    let packed = rel.packed_vector_stats();
    let bindings = rel.archive_vector_stats();
    let profiles = rel.compatibility_profile_stats();
    let generations = rel.vector_generation_stats();
    println!("path: {}", path.display());
    println!("bytes: {}", file_len(path)?);
    println!("kind: reliquary");
    println!("id: {}", rel.owner_id().as_deref().unwrap_or("none"));
    println!("scope: {}", scope_kind_name(rel.scope_kind()));
    println!("legacy_cva: {}", rel.is_legacy_cva());
    println!("archive_version: {}", rel.archive_version());
    println!("vector_version: {}", rel.vector_version());
    println!(
        "archive: nodes={} branches={} fragments={} files={} source_attachments={} file_memory_links={} content_objects={}",
        archive.nodes,
        archive.branches,
        archive.fragments,
        archive.files,
        archive.source_attachments,
        archive.file_memory_links,
        archive.content_objects
    );
    println!(
        "packed_vectors: objects={} rows={} matrix_bytes={}",
        packed.objects, packed.rows, packed.matrix_bytes
    );
    println!(
        "archive_vectors: objects={} rows={}",
        bindings.objects, bindings.rows
    );
    println!("compatibility_profiles: {}", profiles.profiles);
    println!(
        "vector_generations: total={} active_profiles={}",
        generations.generations, generations.active_profiles
    );
    for profile in rel.compatibility_profiles() {
        if let Some(generation) = rel.current_vector_generation(profile.id) {
            println!(
                "active: profile={} generation={} source_archive_version={} vector_version={}",
                hex32(&profile.id.0),
                hex32(&generation.id.0),
                generation.source_archive_version,
                generation.vector_version
            );
        }
    }
    Ok(())
}

fn scope_name(scope: RelScopeArg) -> &'static str {
    match scope {
        RelScopeArg::Organization => "organization",
        RelScopeArg::Project => "project",
        RelScopeArg::Connection => "connection",
    }
}

fn scope_kind_name(scope: ReliquaryScopeKind) -> &'static str {
    match scope {
        ReliquaryScopeKind::Organization => "organization",
        ReliquaryScopeKind::Project => "project",
        ReliquaryScopeKind::Connection => "connection",
    }
}
