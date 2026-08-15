use crate::args::VectorsCommand;
use crate::util::hex32;
use anyhow::Result;
use continuity_memory::Cva;

pub fn run(command: VectorsCommand) -> Result<()> {
    match command {
        VectorsCommand::Status { cva } => status(&cva),
        VectorsCommand::Profiles { cva } => profiles(&cva),
    }
}

fn status(path: &std::path::Path) -> Result<()> {
    let cva = Cva::open(path)?;
    let packed = cva.packed_vector_stats();
    let bindings = cva.archive_vector_stats();
    let generations = cva.vector_generation_stats();
    println!("vector_version: {}", cva.vector_version());
    println!(
        "packed: objects={} rows={} matrix_bytes={}",
        packed.objects, packed.rows, packed.matrix_bytes
    );
    println!(
        "archive_vectors: objects={} rows={}",
        bindings.objects, bindings.rows
    );
    println!(
        "generations: total={} active_profiles={}",
        generations.generations, generations.active_profiles
    );
    for profile in cva.compatibility_profiles() {
        match cva.current_vector_generation(profile.id) {
            Some(generation) => println!(
                "{}\tactive={}\tarchive={}\tvector={}",
                hex32(&profile.id.0),
                hex32(&generation.id.0),
                generation.source_archive_version,
                generation.vector_version
            ),
            None => println!("{}\tno active generation", hex32(&profile.id.0)),
        }
    }
    Ok(())
}

fn profiles(path: &std::path::Path) -> Result<()> {
    let cva = Cva::open(path)?;
    for profile in cva.compatibility_profiles() {
        println!(
            "{}\tdimensions={}\tnormalization={:?}\tprobes={}\tpolicy={}",
            hex32(&profile.id.0),
            profile.dimensions,
            profile.normalization,
            profile.probe_suite_version,
            profile.compatibility_policy_version
        );
    }
    Ok(())
}
