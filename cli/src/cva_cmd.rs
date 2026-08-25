use crate::args::CvaCommand;
use crate::util::{file_len, hex32};
use anyhow::Result;
use reliquary_memory::Cva;

pub fn run(command: CvaCommand) -> Result<()> {
    match command {
        CvaCommand::Create { path } => {
            Cva::create(&path)?;
            println!("created CVA: {}", path.display());
        }
        CvaCommand::Info { path } => info(&path)?,
        CvaCommand::Verify { path } => {
            let cva = Cva::open(&path)?;
            println!(
                "CVA ok: {} archive_version={} vector_version={}",
                path.display(),
                cva.archive_version(),
                cva.vector_version()
            );
        }
    }
    Ok(())
}

fn info(path: &std::path::Path) -> Result<()> {
    let cva = Cva::open(path)?;
    let archive = cva.stats();
    let packed = cva.packed_vector_stats();
    let bindings = cva.archive_vector_stats();
    let profiles = cva.compatibility_profile_stats();
    let generations = cva.vector_generation_stats();
    println!("path: {}", path.display());
    println!("bytes: {}", file_len(path)?);
    println!("archive_version: {}", cva.archive_version());
    println!("vector_version: {}", cva.vector_version());
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
    for profile in cva.compatibility_profiles() {
        if let Some(generation) = cva.current_vector_generation(profile.id) {
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
