use crate::{DreamVerificationVerdict, GraphRelation};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DreamPublicationOutcome {
    NoChange,
    Published(Vec<GraphRelation>),
    Withheld(DreamVerificationVerdict),
}
