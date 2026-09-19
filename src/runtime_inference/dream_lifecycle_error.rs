use crate::MemoryError;
use std::fmt;

#[derive(Debug)]
pub enum DreamLifecycleError {
    Memory(MemoryError),
}

impl fmt::Display for DreamLifecycleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Memory(error) => write!(f, "{error}"),
        }
    }
}

impl std::error::Error for DreamLifecycleError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Memory(error) => Some(error),
        }
    }
}

impl From<MemoryError> for DreamLifecycleError {
    fn from(value: MemoryError) -> Self {
        Self::Memory(value)
    }
}
