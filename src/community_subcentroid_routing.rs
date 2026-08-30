use crate::MemoryId;
use crate::community_routing_bench_fixture::RoutingFixture;
use crate::memory_retrieval_index::build_subcentroids as build_for_snapshot;
use std::collections::HashMap;

pub(crate) use crate::memory_retrieval_index::{
    CommunitySubcentroid as SubcentroidEntry, route_communities as route_subcentroids,
};

pub(crate) fn build_subcentroids(
    fixture: &RoutingFixture,
    vectors: &HashMap<MemoryId, Vec<f32>>,
    count: usize,
) -> Vec<SubcentroidEntry> {
    build_for_snapshot(&fixture.snapshot, vectors, count)
}
