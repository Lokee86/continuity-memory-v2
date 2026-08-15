use crate::{ArchiveError, ArchiveVectorError, ContainerError, PackedVectorError};
use std::fmt;

#[derive(Debug)]
pub enum CvaError {
    Container(ContainerError),
    Archive(ArchiveError),
    PackedVectors(PackedVectorError),
    ArchiveVectors(ArchiveVectorError),
}

impl fmt::Display for CvaError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Container(error) => write!(f, "{error}"),
            Self::Archive(error) => write!(f, "{error}"),
            Self::PackedVectors(error) => write!(f, "{error}"),
            Self::ArchiveVectors(error) => write!(f, "{error}"),
        }
    }
}

impl std::error::Error for CvaError {}

impl From<ContainerError> for CvaError {
    fn from(value: ContainerError) -> Self {
        Self::Container(value)
    }
}

impl From<ArchiveError> for CvaError {
    fn from(value: ArchiveError) -> Self {
        Self::Archive(value)
    }
}

impl From<PackedVectorError> for CvaError {
    fn from(value: PackedVectorError) -> Self {
        Self::PackedVectors(value)
    }
}

impl From<ArchiveVectorError> for CvaError {
    fn from(value: ArchiveVectorError) -> Self {
        Self::ArchiveVectors(value)
    }
}
