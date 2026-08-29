use crate::args::VectorsCommand;
use crate::util::hex32;
use anyhow::Result;
use reliquary_memory::{ConfiguredRuntime, Cva};
use std::path::Path;

pub fn run(config_path: &Path, command: VectorsCommand) -> Result<()> {
    match command {
        VectorsCommand::Status { cva } => status(&cva),
        VectorsCommand::Profiles { cva } => profiles(&cva),
        VectorsCommand::Probe { text } => probe(config_path, &text.join(" ")),
        VectorsCommand::Build {
            cva,
            batch_size,
            concurrency,
        } => build(config_path, &cva, batch_size, concurrency),
    }
}

fn status(path: &Path) -> Result<()> {
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

fn profiles(path: &Path) -> Result<()> {
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

fn probe(config_path: &Path, text: &str) -> Result<()> {
    let report = ConfiguredRuntime::open(config_path)?.probe_embedding(text)?;
    println!(
        "embedding ok: vectors={} dimensions={} normalization={:?}",
        report.vectors, report.dimensions, report.normalization
    );
    Ok(())
}

fn build(config_path: &Path, cva_path: &Path, batch_size: usize, concurrency: usize) -> Result<()> {
    let report = ConfiguredRuntime::open(config_path)?.build_archive_vectors(
        cva_path,
        batch_size,
        concurrency,
    )?;
    println!("profile: {}", hex32(&report.profile_id.0));
    println!("generation: {}", hex32(&report.generation_id.0));
    println!("source_archive_version: {}", report.source_archive_version);
    println!("vector_version: {}", report.vector_version);
    Ok(())
}
