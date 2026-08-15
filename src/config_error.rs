use std::{fmt, io};

#[derive(Debug)]
pub enum ConfigError {
    Io(io::Error),
    InvalidMagic,
    UnsupportedVersion { major: u16, minor: u16 },
    Truncated,
    InvalidObject,
    DuplicateObject(String),
    InvalidFragmentConfig,
    InvalidRetrievalConfig,
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(f, "configuration I/O error: {error}"),
            Self::InvalidMagic => write!(f, "not a Continuity configuration file"),
            Self::UnsupportedVersion { major, minor } => {
                write!(
                    f,
                    "unsupported Continuity configuration format {major}.{minor}"
                )
            }
            Self::Truncated => write!(f, "truncated Continuity configuration file"),
            Self::InvalidObject => write!(f, "invalid Continuity configuration object"),
            Self::DuplicateObject(key) => write!(f, "duplicate configuration object: {key}"),
            Self::InvalidFragmentConfig => write!(f, "invalid fragment configuration"),
            Self::InvalidRetrievalConfig => write!(f, "invalid retrieval configuration"),
        }
    }
}

impl std::error::Error for ConfigError {}

impl From<io::Error> for ConfigError {
    fn from(value: io::Error) -> Self {
        Self::Io(value)
    }
}
