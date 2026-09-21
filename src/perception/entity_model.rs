pub const MAX_ENTITY_NAME_BYTES: usize = 512;
pub const MAX_ENTITY_KIND_BYTES: usize = 128;
pub const MAX_ENTITY_SUMMARY_BYTES: usize = 4096;
pub const MAX_ENTITY_ALIASES: usize = 32;

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct EntityId(pub [u8; 32]);

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EntityDraft {
    pub canonical_name: String,
    pub aliases: Vec<String>,
    pub kind: String,
    pub summary: String,
    pub mutation_id: String,
    pub created_at_ns: i64,
    pub updated_at_ns: i64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Entity {
    pub id: EntityId,
    pub revision: u64,
    pub canonical_name: String,
    pub aliases: Vec<String>,
    pub kind: String,
    pub summary: String,
    pub mutation_id: String,
    pub created_at_ns: i64,
    pub updated_at_ns: i64,
    pub global_version: u64,
    pub entity_version: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EntityStats {
    pub entities: usize,
    pub revisions: usize,
    pub entity_version: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EntityMergeOutcome {
    pub survivor_id: EntityId,
    pub retired_id: EntityId,
    pub aliases_added: usize,
    pub associations_retargeted: usize,
    pub resolutions_retargeted: usize,
    pub changed: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct EntityRecord {
    pub id: EntityId,
    pub revision: u64,
    pub canonical_name: String,
    pub aliases: Vec<String>,
    pub kind: String,
    pub summary: String,
    pub mutation_id: String,
    pub created_at_ns: i64,
    pub updated_at_ns: i64,
    pub global_version: u64,
    pub entity_version: u64,
}
