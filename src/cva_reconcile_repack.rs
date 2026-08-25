use crate::cva_reconcile::{CvaComparison, CvaReconcileResult};
use crate::cva_reconcile_archive::{
    read_archive_tail, replay_archive_tail, replay_file_memory_links,
};
use crate::cva_reconcile_memory::{read_memory_tail, replay_memory_tail};
use crate::{Cva, CvaReconcileError};
use std::fs;
use std::path::Path;

pub(crate) fn reconcile_diverged(
    left_path: &Path,
    right_path: &Path,
    output_path: &Path,
    comparison: CvaComparison,
) -> Result<CvaReconcileResult, CvaReconcileError> {
    let mut left = Cva::open(left_path)?;
    let mut right = Cva::open(right_path)?;
    let metadata = left
        .workspace_metadata()
        .cloned()
        .ok_or(CvaReconcileError::MissingWorkspaceMetadata("left"))?;
    let vector_rebuild_required =
        has_derived_vector_state(&left) || has_derived_vector_state(&right);

    let left_archive = read_archive_tail(&mut left, 0)?;
    let right_archive = read_archive_tail(&mut right, comparison.common_chunk_count)?;
    let left_memory = read_memory_tail(&mut left, 0)?;
    let right_memory = read_memory_tail(&mut right, comparison.common_chunk_count)?;
    let left_profiles = left.compatibility_profiles();
    let right_profiles = right.compatibility_profiles();

    let merge_result = (|| {
        let mut output = Cva::create_workspace(output_path, metadata)?;
        replay_profiles(&mut output, left_profiles, right_profiles)?;
        replay_archive_tail(&mut output, &left_archive)?;
        let archive_records = replay_archive_tail(&mut output, &right_archive)?;
        replay_memory_tail(&mut output, left_memory)?;
        let memory_result = replay_memory_tail(&mut output, right_memory)?;
        replay_file_memory_links(&mut output, &left_archive.file_memory_links)?;
        let file_memory_links =
            replay_file_memory_links(&mut output, &right_archive.file_memory_links)?;
        output.sync()?;
        drop(output);
        Cva::open(output_path)?;
        Ok((archive_records, memory_result, file_memory_links))
    })();

    let (archive_records, memory_result, file_memory_links) = match merge_result {
        Ok(value) => value,
        Err(error) => {
            let _ = fs::remove_file(output_path);
            return Err(error);
        }
    };
    Ok(CvaReconcileResult {
        comparison,
        replayed_archive_records: archive_records,
        replayed_memory_revisions: memory_result.revisions,
        duplicate_memory_revisions: memory_result.duplicate_revisions,
        replayed_insomnia_completions: memory_result.completions,
        replayed_file_memory_links: file_memory_links,
        vector_rebuild_required,
    })
}

fn replay_profiles(
    output: &mut Cva,
    left: Vec<crate::CompatibilityProfile>,
    right: Vec<crate::CompatibilityProfile>,
) -> Result<(), CvaReconcileError> {
    for profile in left.into_iter().chain(right) {
        output
            .compatibility_profiles
            .put(&mut output.container, profile)?;
    }
    Ok(())
}

fn has_derived_vector_state(cva: &Cva) -> bool {
    cva.packed_vector_stats().objects > 0
        || cva.memory_vector_stats().objects > 0
        || cva.archive_vector_stats().objects > 0
        || cva.vector_generation_stats().generations > 0
}
