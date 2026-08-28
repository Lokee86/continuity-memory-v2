use crate::community_scan_merge::scan_merge_snapshot;
use crate::{
    COMMUNITY_ALGORITHM_VERSION, CommunityError, CommunitySnapshot, CommunityStats, Phylactery,
};

impl Phylactery {
    pub fn community_snapshot(&self) -> Option<CommunitySnapshot> {
        self.communities.latest().cloned()
    }

    pub fn community_stats(&self) -> CommunityStats {
        self.communities.stats(self.graph.graph_version())
    }

    pub fn refresh_communities_leiden(&mut self) -> Result<CommunitySnapshot, CommunityError> {
        if let Some(snapshot) = self.communities.latest()
            && snapshot.derived_graph_version == self.graph.graph_version()
            && snapshot.algorithm_version == COMMUNITY_ALGORITHM_VERSION
        {
            return Ok(snapshot.clone());
        }
        let owner_uuid = self
            .owner_uuid()
            .ok_or(CommunityError::MissingOwnerIdentity)?;
        let generation = self.communities.next_generation()?;
        let snapshot = scan_merge_snapshot(&self.graph, owner_uuid, generation)?;
        self.communities.publish(
            &mut self.container,
            &self.graph,
            owner_uuid,
            snapshot.clone(),
        )?;
        Ok(snapshot)
    }
}
