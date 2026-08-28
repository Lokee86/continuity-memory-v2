use crate::MemoryId;

pub const COMMUNITY_ALGORITHM_VERSION: u32 = 1;
pub const COMMUNITY_LEIDEN_SEED: u64 = 0x4c45_4944_454e_0001;
pub const COMMUNITY_LEIDEN_RESOLUTION: f64 = 1.0;

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct CommunityId(pub [u8; 32]);

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Community {
    pub id: CommunityId,
    pub members: Vec<MemoryId>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CommunitySnapshot {
    pub generation: u64,
    pub derived_graph_version: u64,
    pub algorithm_version: u32,
    pub seed: u64,
    pub resolution: f64,
    pub quality: f64,
    pub communities: Vec<Community>,
}

impl CommunitySnapshot {
    pub fn community_for(&self, memory_id: MemoryId) -> Option<&Community> {
        self.communities
            .iter()
            .find(|community| community.members.contains(&memory_id))
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CommunityStats {
    pub generations: u64,
    pub communities: usize,
    pub memberships: usize,
    pub derived_graph_version: Option<u64>,
    pub current: bool,
}
