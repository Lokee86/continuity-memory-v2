use crate::Cva;
use crate::cva_reconcile_archive::{read_archive_tail, replay_archive_tail};
use crate::cva_reconcile_error::CvaReconcileError;
use std::fs;
use std::path::Path;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CvaRelation {
    Identical,
    LeftExtendsRight,
    RightExtendsLeft,
    Diverged,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CvaComparison {
    pub workspace_id: String,
    pub common_chunk_count: usize,
    pub left_chunk_count: usize,
    pub right_chunk_count: usize,
    pub relation: CvaRelation,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CvaReconcileResult {
    pub comparison: CvaComparison,
    pub replayed_archive_records: usize,
    pub skipped_derived_archive_records: usize,
}

impl Cva {
    pub fn compare(
        left_path: impl AsRef<Path>,
        right_path: impl AsRef<Path>,
    ) -> Result<CvaComparison, CvaReconcileError> {
        let mut left = Self::open(left_path)?;
        let mut right = Self::open(right_path)?;
        let left_id = workspace_id(&left, "left")?;
        let right_id = workspace_id(&right, "right")?;
        if left_id != right_id {
            return Err(CvaReconcileError::WorkspaceMismatch {
                left: left_id,
                right: right_id,
            });
        }

        let left_chunks = left.container.chunks()?;
        let right_chunks = right.container.chunks()?;
        let mut common_chunk_count = 0;
        for (left_chunk, right_chunk) in left_chunks.iter().zip(&right_chunks) {
            if left.container.read(*left_chunk)? != right.container.read(*right_chunk)? {
                break;
            }
            common_chunk_count += 1;
        }
        let left_chunk_count = left_chunks.len();
        let right_chunk_count = right_chunks.len();
        let relation = classify(common_chunk_count, left_chunk_count, right_chunk_count);
        Ok(CvaComparison {
            workspace_id: left_id,
            common_chunk_count,
            left_chunk_count,
            right_chunk_count,
            relation,
        })
    }

    pub fn reconcile(
        left_path: impl AsRef<Path>,
        right_path: impl AsRef<Path>,
        output_path: impl AsRef<Path>,
    ) -> Result<CvaReconcileResult, CvaReconcileError> {
        let left_path = left_path.as_ref();
        let right_path = right_path.as_ref();
        let output_path = output_path.as_ref();
        if output_path.exists() {
            return Err(CvaReconcileError::OutputExists);
        }

        let comparison = Self::compare(left_path, right_path)?;
        let source = match comparison.relation {
            CvaRelation::RightExtendsLeft => right_path,
            _ => left_path,
        };
        if comparison.relation != CvaRelation::Diverged {
            copy_and_validate(source, output_path)?;
            return Ok(CvaReconcileResult {
                comparison,
                replayed_archive_records: 0,
                skipped_derived_archive_records: 0,
            });
        }

        let mut right = Self::open(right_path)?;
        let tail = read_archive_tail(&mut right, comparison.common_chunk_count)?;
        fs::copy(left_path, output_path)?;
        let merge_result = (|| {
            let mut output = Self::open(output_path)?;
            let (replayed, skipped) = replay_archive_tail(&mut output, tail)?;
            output.sync()?;
            drop(output);
            Self::open(output_path)?;
            Ok((replayed, skipped))
        })();
        let (replayed, skipped) = match merge_result {
            Ok(value) => value,
            Err(error) => {
                let _ = fs::remove_file(output_path);
                return Err(error);
            }
        };
        Ok(CvaReconcileResult {
            comparison,
            replayed_archive_records: replayed,
            skipped_derived_archive_records: skipped,
        })
    }
}

fn workspace_id(cva: &Cva, side: &'static str) -> Result<String, CvaReconcileError> {
    cva.workspace_metadata()
        .map(|metadata| metadata.id.clone())
        .ok_or(CvaReconcileError::MissingWorkspaceMetadata(side))
}

fn classify(common: usize, left: usize, right: usize) -> CvaRelation {
    if common == left && common == right {
        CvaRelation::Identical
    } else if common == right {
        CvaRelation::LeftExtendsRight
    } else if common == left {
        CvaRelation::RightExtendsLeft
    } else {
        CvaRelation::Diverged
    }
}

fn copy_and_validate(source: &Path, output: &Path) -> Result<(), CvaReconcileError> {
    fs::copy(source, output)?;
    if let Err(error) = Cva::open(output) {
        let _ = fs::remove_file(output);
        return Err(error.into());
    }
    Ok(())
}
