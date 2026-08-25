use crate::{CvaError, InsomniaError};
use std::fmt;

#[derive(Debug)]
pub enum InteractionError {
    Cva(CvaError),
    Insomnia(InsomniaError),
    InvalidField(&'static str),
    UnknownSession,
    SessionAlreadyOpen,
    SessionHasNoDurableTurn,
    MissingResumeMessage,
    ResumeRequired,
    MessageInProgress,
    NoMessageInProgress,
    MessageMismatch,
    MessageAlreadyDurable,
}

impl fmt::Display for InteractionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Cva(error) => write!(f, "{error}"),
            Self::Insomnia(error) => write!(f, "{error}"),
            Self::InvalidField(field) => write!(f, "invalid interaction field: {field}"),
            Self::UnknownSession => write!(f, "interaction session is not open"),
            Self::SessionAlreadyOpen => write!(f, "interaction session is already open"),
            Self::SessionHasNoDurableTurn => write!(f, "interaction session has no durable turn"),
            Self::MissingResumeMessage => write!(f, "interaction resume message does not exist"),
            Self::ResumeRequired => write!(
                f,
                "existing interaction session requires a durable resume message"
            ),
            Self::MessageInProgress => {
                write!(f, "interaction session already has a message in progress")
            }
            Self::NoMessageInProgress => {
                write!(f, "interaction session has no message in progress")
            }
            Self::MessageMismatch => {
                write!(f, "interaction message does not match the active stream")
            }
            Self::MessageAlreadyDurable => write!(f, "interaction message is already durable"),
        }
    }
}

impl std::error::Error for InteractionError {}

impl From<CvaError> for InteractionError {
    fn from(value: CvaError) -> Self {
        Self::Cva(value)
    }
}

impl From<InsomniaError> for InteractionError {
    fn from(value: InsomniaError) -> Self {
        Self::Insomnia(value)
    }
}
