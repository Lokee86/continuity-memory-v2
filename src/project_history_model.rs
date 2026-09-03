#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ProjectRepositoryKind {
    Lore,
    Git,
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct ProjectRepositoryRef {
    pub kind: ProjectRepositoryKind,
    pub repository_id: String,
    pub project_path: String,
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct ProjectRevisionRef {
    pub repository: ProjectRepositoryRef,
    pub revision: String,
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct ProjectFileRef {
    pub revision: ProjectRevisionRef,
    pub path: String,
    pub content_hash: Option<[u8; 32]>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RelSemanticCut {
    pub global_version: u64,
    pub archive_version: u64,
    pub memory_version: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectRevisionCorrelation {
    pub sequence: u64,
    pub rel_cut: RelSemanticCut,
    pub project_revision: ProjectRevisionRef,
}
