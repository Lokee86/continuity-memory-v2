use crate::CommunityId;
use crate::community_routing_bench_support::{RoutingEntry, route};
use std::time::{Duration, Instant};

const DIMENSIONS: usize = 1_024;
const COMMUNITY_SIZE: usize = 64;
const QUERIES: usize = 64;
const KS: [usize; 3] = [1, 3, 5];

#[test]
fn synthetic_community_routing_scale_benchmark() {
    println!(
        "memories,communities,reps_per_community,reps,metadata_bytes,r1,r3,r5,admit5,avoid5,total_vector_work5,work_reduction5,route_us"
    );
    for memories in [1_024, 8_192, 65_536, 262_144] {
        for reps_per_community in [1, 4, 8] {
            run_case(memories, reps_per_community);
        }
    }
}

fn run_case(memories: usize, reps_per_community: usize) {
    assert_eq!(memories % COMMUNITY_SIZE, 0);
    let communities = memories / COMMUNITY_SIZE;
    let entries = synthetic_entries(communities, reps_per_community);
    let mut hits = [0_usize; 3];
    let mut elapsed = Duration::ZERO;
    for query_index in 0..QUERIES {
        let expected = query_index.wrapping_mul(104_729) % communities;
        let query = synthetic_vector(expected, query_index + 65_536, 0.20);
        let started = Instant::now();
        let ranked = route(&entries, &query, KS[2]);
        elapsed += started.elapsed();
        for (slot, k) in KS.into_iter().enumerate() {
            if ranked[..k.min(ranked.len())].contains(&community(expected)) {
                hits[slot] += 1;
            }
        }
    }
    let admit5 = KS[2].min(communities) * COMMUNITY_SIZE;
    let work5 = entries.len() + admit5;
    println!(
        "{memories},{communities},{reps_per_community},{},{},{:.4},{:.4},{:.4},{:.6},{:.6},{:.6},{:.3},{:.3}",
        entries.len(),
        entries.len() * 64,
        hits[0] as f64 / QUERIES as f64,
        hits[1] as f64 / QUERIES as f64,
        hits[2] as f64 / QUERIES as f64,
        admit5 as f64 / memories as f64,
        1.0 - admit5 as f64 / memories as f64,
        work5 as f64 / memories as f64,
        memories as f64 / work5 as f64,
        elapsed.as_secs_f64() * 1_000_000.0 / QUERIES as f64,
    );
}

fn synthetic_entries(communities: usize, reps_per_community: usize) -> Vec<RoutingEntry> {
    let mut entries = Vec::with_capacity(communities * reps_per_community);
    for community_index in 0..communities {
        for rep in 0..reps_per_community {
            entries.push(RoutingEntry {
                community_id: community(community_index),
                vector: synthetic_vector(community_index, rep, 0.15),
            });
        }
    }
    entries
}

fn synthetic_vector(community_index: usize, sample: usize, noise: f32) -> Vec<f32> {
    (0..DIMENSIONS)
        .map(|dimension| {
            let base = signed(hash(community_index as u64, dimension as u64));
            let jitter = signed(hash(
                (community_index as u64) ^ ((sample as u64).wrapping_mul(0x9e37_79b9)),
                dimension as u64 ^ 0xa5a5_5a5a,
            ));
            base + jitter * noise
        })
        .collect()
}

fn signed(value: u64) -> f32 {
    let unit = ((value >> 40) as u32) as f32 / 16_777_215.0;
    unit * 2.0 - 1.0
}

fn hash(mut left: u64, right: u64) -> u64 {
    left ^= right.wrapping_mul(0x9e37_79b9_7f4a_7c15);
    left ^= left >> 30;
    left = left.wrapping_mul(0xbf58_476d_1ce4_e5b9);
    left ^= left >> 27;
    left = left.wrapping_mul(0x94d0_49bb_1331_11eb);
    left ^ (left >> 31)
}

fn community(index: usize) -> CommunityId {
    let mut bytes = [0_u8; 32];
    bytes[..8].copy_from_slice(&(index as u64).to_le_bytes());
    CommunityId(bytes)
}
