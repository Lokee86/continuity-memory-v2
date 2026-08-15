use crate::vector_generation_store::VectorGenerationStore;
use crate::{Archive, CvaError};

pub(crate) fn validate_semantic_global_versions(
    archive: &Archive,
    vector_generations: &VectorGenerationStore,
) -> Result<(), CvaError> {
    let mut versions: Vec<u64> = archive
        .record_versions()
        .iter()
        .map(|record| record.global_version)
        .chain(
            vector_generations
                .generations()
                .iter()
                .map(|generation| generation.global_version),
        )
        .collect();
    versions.sort_unstable();
    for pair in versions.windows(2) {
        if pair[0] == pair[1] {
            return Err(CvaError::SemanticGlobalVersionConflict(pair[0]));
        }
    }
    Ok(())
}
