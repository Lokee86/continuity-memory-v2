use crate::args::DevCommand;
use crate::util::{hex32, parse_normalization, parse_profile_id};
use anyhow::Result;
use reliquary_memory::{
    Cva, DEFAULT_SEARCH_CANDIDATE_LIMIT, MAX_SEMANTIC_SEARCH_LIMIT, RetrievalConfig,
    SimulatedEmbeddingEndpoint,
};

pub fn run(command: DevCommand) -> Result<()> {
    match command {
        DevCommand::EstablishProfile {
            cva,
            dimensions,
            normalization,
            seed,
            drift,
        } => establish(&cva, dimensions, &normalization, seed, drift),
        DevCommand::BuildVectors {
            cva,
            profile,
            seed,
            drift,
        } => build(&cva, &profile, seed, drift),
        DevCommand::Search {
            cva,
            profile,
            seed,
            drift,
            semantic_only,
            limit,
            query,
        } => search(
            &cva,
            &profile,
            seed,
            drift,
            semantic_only,
            limit,
            &query.join(" "),
        ),
    }
}

fn establish(
    path: &std::path::Path,
    dimensions: u32,
    normalization: &str,
    seed: u64,
    drift: f32,
) -> Result<()> {
    let mut cva = Cva::open(path)?;
    let endpoint =
        SimulatedEmbeddingEndpoint::new(dimensions, parse_normalization(normalization)?, seed)
            .with_drift(drift);
    let profile = cva.establish_compatibility_profile(&endpoint)?;
    cva.sync()?;
    println!("profile: {}", hex32(&profile.id.0));
    println!("dimensions: {}", profile.dimensions);
    println!("normalization: {:?}", profile.normalization);
    Ok(())
}

fn build(path: &std::path::Path, profile_text: &str, seed: u64, drift: f32) -> Result<()> {
    let profile_id = parse_profile_id(profile_text)?;
    let mut cva = Cva::open(path)?;
    let profile = cva.compatibility_profile(profile_id)?;
    let endpoint = SimulatedEmbeddingEndpoint::new(profile.dimensions, profile.normalization, seed)
        .with_drift(drift);
    let generation = cva.build_archive_vector_generation(profile_id, &endpoint)?;
    cva.sync()?;
    println!("generation: {}", hex32(&generation.id.0));
    println!(
        "source_archive_version: {}",
        generation.source_archive_version
    );
    println!("vector_version: {}", generation.vector_version);
    Ok(())
}

fn search(
    path: &std::path::Path,
    profile_text: &str,
    seed: u64,
    drift: f32,
    semantic_only: bool,
    limit: usize,
    query: &str,
) -> Result<()> {
    let profile_id = parse_profile_id(profile_text)?;
    let mut cva = Cva::open(path)?;
    let profile = cva.compatibility_profile(profile_id)?;
    let endpoint = SimulatedEmbeddingEndpoint::new(profile.dimensions, profile.normalization, seed)
        .with_drift(drift);
    if semantic_only {
        for hit in cva.semantic_search(profile_id, &endpoint, query, limit)? {
            println!(
                "{:.6}\t{}\t{}\t{}..{}",
                hit.score,
                hex32(&hit.fragment.id.0),
                hit.fragment.conversation_id,
                hit.fragment.start_node_id,
                hit.fragment.end_node_id
            );
        }
    } else {
        let candidate_limit = DEFAULT_SEARCH_CANDIDATE_LIMIT
            .max(limit)
            .min(MAX_SEMANTIC_SEARCH_LIMIT);
        let config = RetrievalConfig {
            candidate_limit,
            result_limit: limit,
            ..RetrievalConfig::default()
        };
        for hit in cva.search_with_config(profile_id, &endpoint, query, config)? {
            println!(
                "{:.6}\tlex={:.6}\tsem={:.6}\t{}\t{}\t{}..{}",
                hit.combined_score,
                hit.lexical_score,
                hit.semantic_score,
                hex32(&hit.fragment.id.0),
                hit.fragment.conversation_id,
                hit.fragment.start_node_id,
                hit.fragment.end_node_id
            );
        }
    }
    Ok(())
}
