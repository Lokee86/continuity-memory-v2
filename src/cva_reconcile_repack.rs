use crate::cva_reconcile::{CvaComparison, CvaReconcileResult};
use crate::cva_reconcile_archive::{
    read_archive_tail, replay_archive_tail, replay_file_memory_links,
};
use crate::cva_reconcile_graph::{read_graph_tail, reconcile_right_graph_tail, replay_graph_tail};
use crate::cva_reconcile_interaction::{merge_interaction_streams, replay_interaction_streams};
use crate::cva_reconcile_memory::{read_memory_tail, replay_memory_tail};
use crate::{CompatibilityProfile, Cva, CvaReconcileError};
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
    let scope = left.scope_kind();
    let metadata = left
        .workspace_metadata()
        .cloned()
        .ok_or(CvaReconcileError::MissingWorkspaceMetadata("left"))?;
    let derived_vectors_present =
        has_derived_vector_state(&left) || has_derived_vector_state(&right);

    let left_archive = read_archive_tail(&mut left, 0)?;
    let right_archive = read_archive_tail(&mut right, comparison.common_chunk_count)?;
    let left_memory = read_memory_tail(&mut left, 0)?;
    let right_memory = read_memory_tail(&mut right, comparison.common_chunk_count)?;
    let left_graph = read_graph_tail(&mut left, 0)?;
    let left_graph_divergent = read_graph_tail(&mut left, comparison.common_chunk_count)?;
    let right_graph_divergent = read_graph_tail(&mut right, comparison.common_chunk_count)?;
    let left_profiles = left.compatibility_profiles();
    let right_profiles = right.compatibility_profiles();
    let (interaction_streams, right_stream_change) = merge_interaction_streams(
        left.interaction_stream_records(),
        right.interaction_stream_records(),
    )?;

    let merge_result = (|| {
        let mut output = Cva::create_workspace_for_scope(output_path, metadata, scope)?;
        replay_profiles(&mut output, left_profiles)?;
        replay_interaction_streams(&mut output, interaction_streams)?;
        replay_archive_tail(&mut output, &left_archive)?;
        replay_memory_tail(&mut output, left_memory)?;
        replay_graph_tail(&mut output, &left_graph)?;
        replay_file_memory_links(&mut output, &left_archive.file_memory_links)?;

        let before_right = output.container.chunks()?.len();
        replay_profiles(&mut output, right_profiles)?;
        let archive_records = replay_archive_tail(&mut output, &right_archive)?;
        let memory_result = replay_memory_tail(&mut output, right_memory)?;
        let graph_result =
            reconcile_right_graph_tail(&mut output, &left_graph_divergent, &right_graph_divergent)?;
        let file_memory_links =
            replay_file_memory_links(&mut output, &right_archive.file_memory_links)?;
        let canonical_change_required =
            output.container.chunks()?.len() > before_right || right_stream_change;

        output.sync()?;
        drop(output);
        Cva::open(output_path)?;
        Ok((
            archive_records,
            memory_result,
            graph_result,
            file_memory_links,
            canonical_change_required,
        ))
    })();

    let (
        archive_records,
        memory_result,
        graph_result,
        file_memory_links,
        canonical_change_required,
    ) = match merge_result {
        Ok(value) => value,
        Err(error) => {
            let _ = fs::remove_file(output_path);
            return Err(error);
        }
    };

    if !canonical_change_required {
        replace_with_left(left_path, output_path)?;
    }

    Ok(CvaReconcileResult {
        comparison,
        replayed_archive_records: archive_records,
        replayed_memory_revisions: memory_result.revisions,
        duplicate_memory_revisions: memory_result.duplicate_revisions,
        replayed_insomnia_completions: memory_result.completions,
        replayed_graph_transactions: graph_result.transactions,
        replayed_graph_mutations: graph_result.mutations,
        duplicate_graph_mutations: graph_result.duplicate_mutations,
        replayed_file_memory_links: file_memory_links,
        canonical_change_required,
        vector_rebuild_required: canonical_change_required && derived_vectors_present,
    })
}

fn replay_profiles(
    output: &mut Cva,
    profiles: Vec<CompatibilityProfile>,
) -> Result<(), CvaReconcileError> {
    for profile in profiles {
        output
            .compatibility_profiles
            .put(&mut output.container, profile)?;
    }
    Ok(())
}

fn replace_with_left(left_path: &Path, output_path: &Path) -> Result<(), CvaReconcileError> {
    fs::remove_file(output_path)?;
    fs::copy(left_path, output_path)?;
    Cva::open(output_path)?;
    Ok(())
}

fn has_derived_vector_state(cva: &Cva) -> bool {
    cva.packed_vector_stats().objects > 0
        || cva.memory_vector_stats().objects > 0
        || cva.archive_vector_stats().objects > 0
        || cva.vector_generation_stats().generations > 0
}
