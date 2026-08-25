use crate::{ContainerError, Cva, CvaError};
use std::fmt;
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

#[derive(Debug)]
pub enum CvaReconcileError {
    Cva(CvaError),
    Container(ContainerError),
    MissingWorkspaceMetadata(&'static str),
    WorkspaceMismatch { left: String, right: String },
}

impl Cva {
    pub fn compare(
        left_path: impl AsRef<Path>,
        right_path: impl AsRef<Path>,
    ) -> Result<CvaComparison, CvaReconcileError> {
        let mut left = Self::open(left_path)?;
        let mut right = Self::open(right_path)?;

        let left_id = left
            .workspace_metadata()
            .ok_or(CvaReconcileError::MissingWorkspaceMetadata("left"))?
            .id
            .clone();
        let right_id = right
            .workspace_metadata()
            .ok_or(CvaReconcileError::MissingWorkspaceMetadata("right"))?
            .id
            .clone();
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
        let relation =
            if common_chunk_count == left_chunk_count && common_chunk_count == right_chunk_count {
                CvaRelation::Identical
            } else if common_chunk_count == right_chunk_count {
                CvaRelation::LeftExtendsRight
            } else if common_chunk_count == left_chunk_count {
                CvaRelation::RightExtendsLeft
            } else {
                CvaRelation::Diverged
            };

        Ok(CvaComparison {
            workspace_id: left_id,
            common_chunk_count,
            left_chunk_count,
            right_chunk_count,
            relation,
        })
    }
}

impl fmt::Display for CvaReconcileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Cva(error) => write!(f, "{error}"),
            Self::Container(error) => write!(f, "{error}"),
            Self::MissingWorkspaceMetadata(side) => {
                write!(f, "{side} CVA has no workspace metadata")
            }
            Self::WorkspaceMismatch { left, right } => {
                write!(f, "workspace mismatch: left={left} right={right}")
            }
        }
    }
}

impl std::error::Error for CvaReconcileError {}

impl From<CvaError> for CvaReconcileError {
    fn from(value: CvaError) -> Self {
        Self::Cva(value)
    }
}

impl From<ContainerError> for CvaReconcileError {
    fn from(value: ContainerError) -> Self {
        Self::Container(value)
    }
}
