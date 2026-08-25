use crate::MemoryId;
use arcana_graph::{EdgeKind, NodeId};

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum GraphRelationKind {
    Topical,
    Factual,
    Causal,
    Recurrent,
    References,
    DuplicateOf,
    Supersedes,
    StructuralParent,
}

impl GraphRelationKind {
    pub const fn code(self) -> u16 {
        match self {
            Self::Topical => 1,
            Self::Factual => 2,
            Self::Causal => 3,
            Self::Recurrent => 4,
            Self::References => 5,
            Self::DuplicateOf => 6,
            Self::Supersedes => 7,
            Self::StructuralParent => 8,
        }
    }

    pub const fn from_code(code: u16) -> Option<Self> {
        Some(match code {
            1 => Self::Topical,
            2 => Self::Factual,
            3 => Self::Causal,
            4 => Self::Recurrent,
            5 => Self::References,
            6 => Self::DuplicateOf,
            7 => Self::Supersedes,
            8 => Self::StructuralParent,
            _ => return None,
        })
    }

    pub(crate) const fn edge_kind(self) -> EdgeKind {
        EdgeKind(self.code())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GraphDirection {
    Outgoing,
    Incoming,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GraphRelation {
    pub source: MemoryId,
    pub target: MemoryId,
    pub kind: GraphRelationKind,
    pub active: bool,
    pub global_version: u64,
    pub graph_version: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GraphNeighbor {
    pub memory_id: MemoryId,
    pub kind: GraphRelationKind,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MemoryGraphPath {
    pub memories: Vec<MemoryId>,
    pub relations: Vec<GraphRelationKind>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GraphStats {
    pub nodes: usize,
    pub active_relations: usize,
    pub relation_mutations: usize,
    pub graph_version: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct GraphNodeRecord {
    pub memory_id: MemoryId,
    pub node_id: NodeId,
}
