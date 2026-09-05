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
    let legacy_scope = left.legacy_scope_kind();
    let left_rel_metadata = left.rel_metadata();
    let right_rel_metadata = right.rel_metadata();
    if left_rel_metadata != right_rel_metadata {
        return Err(CvaReconcileError::UnsupportedSemanticOwner("REL metadata"));
    }
    let owner_uuid = left
        .owner_uuid()
        .ok_or(CvaReconcileError::MissingOwnerId("left"))?;
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
    let left_echo = left.echo_records();
    let right_echo = right.echo_records();
    let left_project_history = left.project_revision_correlations();
    let right_project_history = right.project_revision_correlations();
    let latest_project_revision =
        compatible_project_revision(&left_project_history, &right_project_history).ok_or(
            CvaReconcileError::UnsupportedSemanticOwner(
                "divergent project-revision correlation history",
            ),
        )?;
    let right_project_history_change = right_project_history.len() > left_project_history.len();
    let (interaction_streams, right_stream_change) = merge_interaction_streams(
        left.interaction_stream_records(),
        right.interaction_stream_records(),
    )?;

    let merge_result = (|| {
        let mut output = match legacy_scope {
            Some(scope) => Cva::create_legacy_scope_with_uuid(output_path, scope, owner_uuid)?,
            None => Cva::create_rel_with_uuid(output_path, owner_uuid, None)?,
        };
        output.set_rel_metadata(
            left_rel_metadata.type_label.clone(),
            left_rel_metadata.dependencies.clone(),
        )?;
        replay_profiles(&mut output, left_profiles)?;
        replay_interaction_streams(&mut output, interaction_streams)?;
        replay_archive_tail(&mut output, &left_archive)?;
        replay_memory_tail(&mut output, left_memory)?;
        replay_graph_tail(&mut output, &left_graph)?;
        replay_file_memory_links(&mut output, &left_archive.file_memory_links)?;
        replay_echo(&mut output, left_echo)?;

        let before_right = output.container.chunks()?.len();
        replay_profiles(&mut output, right_profiles)?;
        let archive_records = replay_archive_tail(&mut output, &right_archive)?;
        let memory_result = replay_memory_tail(&mut output, right_memory)?;
        let graph_result =
            reconcile_right_graph_tail(&mut output, &left_graph_divergent, &right_graph_divergent)?;
        let file_memory_links =
            replay_file_memory_links(&mut output, &right_archive.file_memory_links)?;
        replay_echo(&mut output, right_echo)?;
        let canonical_change_required = output.container.chunks()?.len() > before_right
            || right_stream_change
            || right_project_history_change;
        if canonical_change_required
            && let Some((project_revision, management)) = latest_project_revision.clone()
        {
            output.correlate_project_revision_with_management(project_revision, management)?;
        }

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

fn replay_echo(output: &mut Cva, events: Vec<crate::EchoEvent>) -> Result<(), CvaReconcileError> {
    for event in events {
        output.put_echo_event(event)?;
    }
    Ok(())
}

fn compatible_project_revision(
    left: &[crate::ProjectRevisionCorrelation],
    right: &[crate::ProjectRevisionCorrelation],
) -> Option<
    Option<(
        crate::ProjectRevisionRef,
        crate::ProjectRepositoryManagement,
    )>,
> {
    let common = left
        .iter()
        .zip(right)
        .take_while(|(left, right)| {
            left.project_revision == right.project_revision
                && left.repository_management == right.repository_management
        })
        .count();
    if common != left.len().min(right.len()) {
        return None;
    }
    let history = if right.len() > left.len() {
        right
    } else {
        left
    };
    Some(history.last().map(|record| {
        (
            record.project_revision.clone(),
            record.repository_management,
        )
    }))
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
