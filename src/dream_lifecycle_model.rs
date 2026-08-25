use crate::{Memory, MemoryId};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DreamLifecycleResult {
    pub source: Memory,
    pub revised: Vec<Memory>,
    pub archived: Vec<MemoryId>,
    pub promoted_to_knowledge: bool,
}
