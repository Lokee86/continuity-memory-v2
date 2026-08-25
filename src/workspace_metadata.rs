use crate::ContainerError;
use std::fmt;

pub const MAX_WORKSPACE_ID_BYTES: usize = 256;
pub const MAX_WORKSPACE_NAME_BYTES: usize = 1024;
pub const MAX_WORKSPACE_TYPE_BYTES: usize = 256;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WorkspaceMetadata {
    pub id: String,
    pub name: String,
    pub workspace_type: String,
}

impl WorkspaceMetadata {
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        workspace_type: impl Into<String>,
    ) -> Result<Self, WorkspaceMetadataError> {
        let metadata = Self {
            id: id.into(),
            name: name.into(),
            workspace_type: workspace_type.into(),
        };
        metadata.validate()?;
        Ok(metadata)
    }

    pub(crate) fn validate(&self) -> Result<(), WorkspaceMetadataError> {
        validate_field("id", &self.id, MAX_WORKSPACE_ID_BYTES)?;
        validate_field("name", &self.name, MAX_WORKSPACE_NAME_BYTES)?;
        validate_field(
            "workspace_type",
            &self.workspace_type,
            MAX_WORKSPACE_TYPE_BYTES,
        )?;
        Ok(())
    }
}

fn validate_field(
    field: &'static str,
    value: &str,
    max_bytes: usize,
) -> Result<(), WorkspaceMetadataError> {
    if value.trim().is_empty() {
        return Err(WorkspaceMetadataError::InvalidField(field));
    }
    if value.len() > max_bytes {
        return Err(WorkspaceMetadataError::FieldTooLarge(field));
    }
    Ok(())
}

#[derive(Debug)]
pub enum WorkspaceMetadataError {
    Container(ContainerError),
    AlreadyInitialized,
    InvalidField(&'static str),
    FieldTooLarge(&'static str),
    InvalidUtf8,
    CorruptRecord(&'static str),
    ConflictingFormat,
}

impl fmt::Display for WorkspaceMetadataError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Container(error) => write!(f, "{error}"),
            Self::AlreadyInitialized => write!(f, "workspace metadata is already initialized"),
            Self::InvalidField(field) => write!(f, "workspace metadata field {field} is empty"),
            Self::FieldTooLarge(field) => {
                write!(f, "workspace metadata field {field} is too large")
            }
            Self::InvalidUtf8 => write!(f, "workspace metadata contains invalid UTF-8"),
            Self::CorruptRecord(message) => write!(f, "corrupt workspace metadata: {message}"),
            Self::ConflictingFormat => write!(f, "conflicting workspace metadata format marker"),
        }
    }
}

impl std::error::Error for WorkspaceMetadataError {}

impl From<ContainerError> for WorkspaceMetadataError {
    fn from(value: ContainerError) -> Self {
        Self::Container(value)
    }
}
