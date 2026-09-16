use crate::{
    InsomniaRejection, Memory, MemoryDraft, MemoryRef, MemorySourceRef, TemporalAssessment,
};

pub(crate) struct PreparedApplication {
    pub(crate) project_drafts: Vec<PreparedMemory>,
    pub(crate) user_drafts: Vec<PreparedUserMemory>,
    pub(crate) rejected: Vec<InsomniaRejection>,
    pub(crate) model: String,
}

pub(crate) struct PreparedMemory {
    pub(crate) draft: MemoryDraft,
    pub(crate) temporal: TemporalAssessment,
}

pub(crate) struct PreparedUserMemory {
    pub(crate) memory: PreparedMemory,
    pub(crate) source_ref: MemorySourceRef,
}

pub(crate) struct UserPublication {
    pub(crate) created: Vec<Memory>,
    pub(crate) existing: Vec<Memory>,
    pub(crate) refs: Vec<MemoryRef>,
}
