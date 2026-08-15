use crate::{
    ArchiveError, ArchiveVectorError, CompatibilityProfileError, ContainerError, InsomniaError,
    MemoryError, MemoryVectorError, PackedVectorError, VectorGenerationError,
};
use std::fmt;

#[derive(Debug)]
pub enum CvaError {
    Container(ContainerError),
    Archive(ArchiveError),
    Memories(MemoryError),
    Insomnia(InsomniaError),
    PackedVectors(PackedVectorError),
    MemoryVectors(MemoryVectorError),
    ArchiveVectors(ArchiveVectorError),
    CompatibilityProfiles(CompatibilityProfileError),
    VectorGenerations(VectorGenerationError),
    SemanticGlobalVersionConflict(u64),
}

impl fmt::Display for CvaError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Container(error) => write!(f, "{error}"),
            Self::Archive(error) => write!(f, "{error}"),
            Self::Memories(error) => write!(f, "{error}"),
            Self::Insomnia(error) => write!(f, "{error}"),
            Self::PackedVectors(error) => write!(f, "{error}"),
            Self::MemoryVectors(error) => write!(f, "{error}"),
            Self::ArchiveVectors(error) => write!(f, "{error}"),
            Self::CompatibilityProfiles(error) => write!(f, "{error}"),
            Self::VectorGenerations(error) => write!(f, "{error}"),
            Self::SemanticGlobalVersionConflict(version) => {
                write!(
                    f,
                    "CVA global version {version} is claimed by multiple semantic mutations"
                )
            }
        }
    }
}

impl std::error::Error for CvaError {}

macro_rules! from_error {
    ($source:ty, $variant:ident) => {
        impl From<$source> for CvaError {
            fn from(value: $source) -> Self {
                Self::$variant(value)
            }
        }
    };
}

from_error!(ContainerError, Container);
from_error!(ArchiveError, Archive);
from_error!(MemoryError, Memories);
from_error!(InsomniaError, Insomnia);
from_error!(PackedVectorError, PackedVectors);
from_error!(MemoryVectorError, MemoryVectors);
from_error!(ArchiveVectorError, ArchiveVectors);
from_error!(CompatibilityProfileError, CompatibilityProfiles);
from_error!(VectorGenerationError, VectorGenerations);
