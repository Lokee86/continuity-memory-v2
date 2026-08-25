use crate::{ArchiveError, ContainerError, CvaError, InsomniaError, MemoryError};
use std::fmt;
use std::io;

#[derive(Debug)]
pub enum CvaReconcileError {
    Cva(CvaError),
    Container(ContainerError),
    Archive(ArchiveError),
    Memories(MemoryError),
    Insomnia(InsomniaError),
    Io(io::Error),
    MissingWorkspaceMetadata(&'static str),
    WorkspaceMismatch { left: String, right: String },
    OutputExists,
    UnsupportedSemanticOwner(&'static str),
    InvalidInsomniaCompletion(&'static str),
    InvalidMemoryVersionRecord,
    ConflictingInsomniaCompletion,
    MissingCompletionMemory,
}

impl fmt::Display for CvaReconcileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Cva(error) => write!(f, "{error}"),
            Self::Container(error) => write!(f, "{error}"),
            Self::Archive(error) => write!(f, "{error}"),
            Self::Memories(error) => write!(f, "{error}"),
            Self::Insomnia(error) => write!(f, "{error}"),
            Self::Io(error) => write!(f, "{error}"),
            Self::MissingWorkspaceMetadata(side) => {
                write!(f, "{side} CVA has no workspace metadata")
            }
            Self::WorkspaceMismatch { left, right } => {
                write!(f, "workspace mismatch: left={left} right={right}")
            }
            Self::OutputExists => write!(f, "reconciliation output already exists"),
            Self::UnsupportedSemanticOwner(owner) => {
                write!(f, "reconciliation does not yet support divergent {owner}")
            }
            Self::InvalidInsomniaCompletion(message) => {
                write!(f, "invalid Insomnia completion: {message}")
            }
            Self::InvalidMemoryVersionRecord => {
                write!(f, "memory version points at a non-memory record")
            }
            Self::ConflictingInsomniaCompletion => {
                write!(f, "divergent Insomnia completions conflict")
            }
            Self::MissingCompletionMemory => {
                write!(f, "Insomnia completion references a missing Memory")
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

impl From<ArchiveError> for CvaReconcileError {
    fn from(value: ArchiveError) -> Self {
        Self::Archive(value)
    }
}

impl From<MemoryError> for CvaReconcileError {
    fn from(value: MemoryError) -> Self {
        Self::Memories(value)
    }
}

impl From<InsomniaError> for CvaReconcileError {
    fn from(value: InsomniaError) -> Self {
        Self::Insomnia(value)
    }
}

impl From<io::Error> for CvaReconcileError {
    fn from(value: io::Error) -> Self {
        Self::Io(value)
    }
}
