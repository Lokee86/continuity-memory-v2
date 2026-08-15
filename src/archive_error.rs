use crate::ContainerError;
use std::fmt;

#[derive(Debug)]
pub enum ArchiveError {
    Container(ContainerError),
    InvalidField(&'static str),
    CorruptRecord(&'static str),
    FieldTooLarge,
    InvalidUtf8,
    MissingParent,
    MissingLeaf,
    MissingBranch,
    MissingNode,
    MissingContent,
    ConflictingNode,
    InvalidBranchRevision,
    MissingFragment,
    ConflictingFragment,
    InvalidFragmentConfig,
    InvalidFragmentId,
    InvalidFragmentRange,
    MissingEpisodeNode,
    ConflictingEpisode,
    InvalidEpisodeConfig,
    InvalidEpisodeRange,
    InvalidEpisodePayload,
    EpisodeBranchConflict,
    NodeCycle,
    HashCollision,
    CorruptContent,
    MissingArchiveFormat,
    ConflictingArchiveFormat,
    InvalidArchiveRecordVersion,
    ArchiveVersionExhausted,
}

impl fmt::Display for ArchiveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "archive error: {self:?}")
    }
}

impl std::error::Error for ArchiveError {}

impl From<ContainerError> for ArchiveError {
    fn from(value: ContainerError) -> Self {
        Self::Container(value)
    }
}
