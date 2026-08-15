use continuity_memory::{
    Cva, EmbeddingEndpointDescriptor, SimulatedEmbeddingEndpoint, VectorNormalization,
};
use std::error::Error;
use std::fs;
use std::path::PathBuf;

fn endpoint(seed: u64) -> SimulatedEmbeddingEndpoint {
    SimulatedEmbeddingEndpoint::new(
        EmbeddingEndpointDescriptor {
            provider: "simulated".into(),
            model: "corpus-smoke".into(),
            revision: "r1".into(),
            dimensions: 32,
            normalization: VectorNormalization::L2,
        },
        seed,
    )
}

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = std::env::args().skip(1);
    let source = PathBuf::from(args.next().ok_or("missing source .cva")?);
    let output = PathBuf::from(args.next().ok_or("missing output .cva")?);
    if output.exists() {
        fs::remove_file(&output)?;
    }
    fs::copy(&source, &output)?;

    let mut cva = Cva::open(&output)?;
    let rows = cva.fragments().len() as u64;
    let mut expected = Vec::new();
    for seed in [101_u64, 202_u64] {
        let endpoint = endpoint(seed);
        let profile = cva.create_embedding_profile(&endpoint)?;
        let generation = cva.build_archive_vector_generation(profile.id, &endpoint)?;
        expected.push((profile.id, generation));
    }
    cva.sync()?;
    drop(cva);

    let reopened = Cva::open(&output)?;
    for (profile_id, generation) in expected {
        assert_eq!(
            reopened.current_vector_generation(profile_id),
            Some(generation)
        );
    }
    assert_eq!(reopened.embedding_profile_stats().profiles, 2);
    assert_eq!(reopened.vector_generation_stats().generations, 2);
    assert_eq!(reopened.vector_generation_stats().active_profiles, 2);
    assert_eq!(reopened.archive_vector_stats().rows, rows * 2);
    println!(
        "vector smoke ok: {} fragments, {} profiles, {} generations, {} vector rows, {} bytes",
        rows,
        reopened.embedding_profile_stats().profiles,
        reopened.vector_generation_stats().generations,
        reopened.archive_vector_stats().rows,
        fs::metadata(&output)?.len()
    );
    Ok(())
}
