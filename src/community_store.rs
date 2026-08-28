use crate::community_codec::{decode_snapshot, encode_snapshot};
use crate::graph_store::GraphStore;
use crate::{
    COMMUNITY_ALGORITHM_VERSION, CommunityError, CommunityId, CommunitySnapshot, CommunityStats,
    Container, MemoryId,
};
use sha2::{Digest, Sha256};
use std::collections::HashSet;

#[derive(Default)]
pub(crate) struct CommunityStore {
    snapshots: Vec<CommunitySnapshot>,
}

impl CommunityStore {
    pub(crate) fn latest(&self) -> Option<&CommunitySnapshot> {
        self.snapshots.last()
    }

    pub(crate) fn next_generation(&self) -> Result<u64, CommunityError> {
        self.latest().map_or(Ok(1), |snapshot| {
            snapshot
                .generation
                .checked_add(1)
                .ok_or(CommunityError::GenerationOverflow)
        })
    }

    pub(crate) fn publish(
        &mut self,
        container: &mut Container,
        graph: &GraphStore,
        owner_uuid: [u8; 16],
        snapshot: CommunitySnapshot,
    ) -> Result<(), CommunityError> {
        self.validate_next(graph, owner_uuid, &snapshot)?;
        container.append(&encode_snapshot(&snapshot)?)?;
        self.snapshots.push(snapshot);
        Ok(())
    }

    pub(crate) fn stats(&self, graph_version: u64) -> CommunityStats {
        let latest = self.latest();
        CommunityStats {
            generations: latest.map_or(0, |snapshot| snapshot.generation),
            communities: latest.map_or(0, |snapshot| snapshot.communities.len()),
            memberships: latest.map_or(0, |snapshot| {
                snapshot
                    .communities
                    .iter()
                    .map(|community| community.members.len())
                    .sum()
            }),
            derived_graph_version: latest.map(|snapshot| snapshot.derived_graph_version),
            current: latest.is_some_and(|snapshot| {
                snapshot.derived_graph_version == graph_version
                    && snapshot.algorithm_version == COMMUNITY_ALGORITHM_VERSION
            }),
        }
    }

    fn insert_rebuilt(
        &mut self,
        graph: &GraphStore,
        owner_uuid: [u8; 16],
        snapshot: CommunitySnapshot,
    ) -> Result<(), CommunityError> {
        self.validate_next(graph, owner_uuid, &snapshot)?;
        self.snapshots.push(snapshot);
        Ok(())
    }

    fn validate_next(
        &self,
        graph: &GraphStore,
        owner_uuid: [u8; 16],
        snapshot: &CommunitySnapshot,
    ) -> Result<(), CommunityError> {
        if snapshot.generation != self.next_generation()? {
            return Err(CommunityError::InvalidSnapshot("non-contiguous generation"));
        }
        validate_snapshot(graph, owner_uuid, snapshot)
    }
}

pub(crate) struct CommunityOpenState {
    snapshots: Vec<CommunitySnapshot>,
}

impl CommunityOpenState {
    pub(crate) fn new() -> Self {
        Self {
            snapshots: Vec::new(),
        }
    }

    pub(crate) fn ingest(&mut self, payload: &[u8]) -> Result<(), CommunityError> {
        if let Some(snapshot) = decode_snapshot(payload)? {
            self.snapshots.push(snapshot);
        }
        Ok(())
    }

    pub(crate) fn finish(
        self,
        graph: &GraphStore,
        owner_uuid: Option<[u8; 16]>,
    ) -> Result<CommunityStore, CommunityError> {
        if self.snapshots.is_empty() {
            return Ok(CommunityStore::default());
        }
        let owner_uuid = owner_uuid.ok_or(CommunityError::MissingOwnerIdentity)?;
        let mut store = CommunityStore::default();
        for snapshot in self.snapshots {
            store.insert_rebuilt(graph, owner_uuid, snapshot)?;
        }
        Ok(store)
    }
}

pub(crate) fn community_id(owner_uuid: [u8; 16], members: &[MemoryId]) -> CommunityId {
    let mut hash = Sha256::new();
    hash.update(b"reliquary-community-v1\0");
    hash.update(owner_uuid);
    hash.update((members.len() as u64).to_le_bytes());
    for member in members {
        hash.update(member.0);
    }
    CommunityId(hash.finalize().into())
}

fn validate_snapshot(
    graph: &GraphStore,
    owner_uuid: [u8; 16],
    snapshot: &CommunitySnapshot,
) -> Result<(), CommunityError> {
    if snapshot.algorithm_version == 0 || snapshot.algorithm_version > COMMUNITY_ALGORITHM_VERSION {
        return Err(CommunityError::InvalidSnapshot(
            "unsupported algorithm version",
        ));
    }
    if !snapshot.resolution.is_finite() || snapshot.resolution <= 0.0 {
        return Err(CommunityError::InvalidSnapshot("invalid resolution"));
    }
    if !snapshot.quality.is_finite() {
        return Err(CommunityError::InvalidSnapshot("invalid quality"));
    }
    if snapshot.derived_graph_version > graph.graph_version() {
        return Err(CommunityError::InvalidSnapshot("future graph version"));
    }

    let mut previous_community = None;
    let mut seen_members = HashSet::new();
    for community in &snapshot.communities {
        if community.members.is_empty() {
            return Err(CommunityError::InvalidSnapshot("empty community"));
        }
        if previous_community.is_some_and(|previous| previous >= community.id) {
            return Err(CommunityError::InvalidSnapshot("community order"));
        }
        previous_community = Some(community.id);

        let mut previous_member: Option<[u8; 32]> = None;
        for member in &community.members {
            if previous_member.is_some_and(|previous| previous >= member.0) {
                return Err(CommunityError::InvalidSnapshot("member order"));
            }
            previous_member = Some(member.0);
            graph.node_id(*member)?;
            if !seen_members.insert(*member) {
                return Err(CommunityError::InvalidSnapshot("duplicate member"));
            }
        }
        if community.id != community_id(owner_uuid, &community.members) {
            return Err(CommunityError::InvalidSnapshot("community identity"));
        }
    }

    if snapshot.derived_graph_version == graph.graph_version()
        && seen_members.len() != graph.stats().nodes
    {
        return Err(CommunityError::InvalidSnapshot(
            "current snapshot does not cover graph nodes",
        ));
    }
    Ok(())
}
