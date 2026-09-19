use crate::CommunityId;
use crate::community_routing_bench_fixture::{RoutingFixture, RoutingProbe, load_fixture};
use crate::community_routing_bench_support::{BenchStrategy, entries, logical_index_bytes, route};
use std::collections::HashSet;
use std::time::{Duration, Instant};

const KS: [usize; 3] = [1, 3, 5];

#[test]
fn community_routing_strategy_benchmark() {
    let fixture = load_fixture();
    run_fixture(
        "configured-project-rel",
        &fixture,
        &[
            BenchStrategy::Centroid,
            BenchStrategy::Medoid,
            BenchStrategy::Diverse(4),
            BenchStrategy::Diverse(8),
            BenchStrategy::Structural(4),
            BenchStrategy::Structural(8),
        ],
    );
}

#[test]
fn community_routing_scalable_strategy_benchmark() {
    let fixture = load_fixture();
    run_fixture(
        "configured-project-rel-scalable",
        &fixture,
        &[
            BenchStrategy::Centroid,
            BenchStrategy::Structural(1),
            BenchStrategy::Structural(2),
            BenchStrategy::Structural(4),
            BenchStrategy::Structural(8),
        ],
    );
}

fn run_fixture(label: &str, fixture: &RoutingFixture, strategies: &[BenchStrategy]) {
    println!(
        "fixture={label} vectors={} graph_memories={} residual_memories={} communities={} gold_queries={} gold_targets={} semantic_queries={} semantic_targets={}",
        fixture.vectors.len(),
        fixture.memberships.len(),
        fixture
            .vectors
            .len()
            .saturating_sub(fixture.memberships.len()),
        fixture.snapshot.communities.len(),
        fixture.gold.len(),
        target_count(&fixture.gold),
        fixture.semantic.len(),
        target_count(&fixture.semantic),
    );
    println!(
        "strategy,reps,index_bytes,gold_r1,gold_r3,gold_r5,semantic_r1,semantic_r3,semantic_r5,admit1,admit3,admit5,avoid5,route_us"
    );
    let gold_oracle = evaluate_oracle(fixture, &fixture.gold);
    let semantic_oracle = evaluate_oracle(fixture, &fixture.semantic);
    println!(
        "oracle,0,0,{:.4},{:.4},{:.4},{:.4},{:.4},{:.4},{:.4},{:.4},{:.4},{:.4},0.000",
        gold_oracle.recall[0],
        gold_oracle.recall[1],
        gold_oracle.recall[2],
        semantic_oracle.recall[0],
        semantic_oracle.recall[1],
        semantic_oracle.recall[2],
        semantic_oracle.admitted[0],
        semantic_oracle.admitted[1],
        semantic_oracle.admitted[2],
        1.0 - semantic_oracle.admitted[2],
    );
    for &strategy in strategies {
        let gold = evaluate(fixture, strategy, &fixture.gold);
        let semantic = evaluate(fixture, strategy, &fixture.semantic);
        let reps = entries(fixture, strategy, None).len();
        println!(
            "{},{reps},{},{:.4},{:.4},{:.4},{:.4},{:.4},{:.4},{:.4},{:.4},{:.4},{:.4},{:.3}",
            strategy.name(),
            logical_index_bytes(fixture, strategy),
            gold.recall[0],
            gold.recall[1],
            gold.recall[2],
            semantic.recall[0],
            semantic.recall[1],
            semantic.recall[2],
            semantic.admitted[0],
            semantic.admitted[1],
            semantic.admitted[2],
            1.0 - semantic.admitted[2],
            semantic.route_time.as_secs_f64() * 1_000_000.0 / semantic.queries as f64,
        );
    }
}

struct Evaluation {
    recall: [f64; 3],
    admitted: [f64; 3],
    route_time: Duration,
    queries: usize,
}

fn evaluate(
    fixture: &RoutingFixture,
    strategy: BenchStrategy,
    probes: &[RoutingProbe],
) -> Evaluation {
    let mut hits = [0_usize; 3];
    let mut admitted = [0.0_f64; 3];
    let targets = target_count(probes);
    let mut route_time = Duration::ZERO;
    for probe in probes {
        let routing_entries = entries(fixture, strategy, Some(probe.query));
        let query = fixture.vectors.get(&probe.query).unwrap();
        let started = Instant::now();
        let ranked = route(&routing_entries, query, KS[2]);
        route_time += started.elapsed();
        for (slot, k) in KS.into_iter().enumerate() {
            let selected = &ranked[..k.min(ranked.len())];
            hits[slot] += probe
                .targets
                .iter()
                .filter(|target| match fixture.memberships.get(target) {
                    Some(community) => selected.contains(community),
                    None => true,
                })
                .count();
            admitted[slot] += admitted_fraction(fixture, selected, probe.query);
        }
    }
    Evaluation {
        recall: hits.map(|hit| hit as f64 / targets.max(1) as f64),
        admitted: admitted.map(|sum| sum / probes.len().max(1) as f64),
        route_time,
        queries: probes.len().max(1),
    }
}

fn evaluate_oracle(fixture: &RoutingFixture, probes: &[RoutingProbe]) -> Evaluation {
    let mut hits = [0_usize; 3];
    let mut admitted = [0.0_f64; 3];
    let targets = target_count(probes);
    for probe in probes {
        let mut counts = std::collections::HashMap::<CommunityId, usize>::new();
        let mut residual = 0_usize;
        for target in &probe.targets {
            match fixture.memberships.get(target) {
                Some(community) => *counts.entry(*community).or_default() += 1,
                None => residual += 1,
            }
        }
        let mut ranked: Vec<_> = counts.into_iter().collect();
        ranked.sort_by(|left, right| right.1.cmp(&left.1).then_with(|| left.0.cmp(&right.0)));
        for (slot, k) in KS.into_iter().enumerate() {
            let selected: Vec<_> = ranked
                .iter()
                .take(k)
                .map(|(community, _)| *community)
                .collect();
            hits[slot] += residual
                + ranked
                    .iter()
                    .take(k)
                    .map(|(_, count)| *count)
                    .sum::<usize>();
            admitted[slot] += admitted_fraction(fixture, &selected, probe.query);
        }
    }
    Evaluation {
        recall: hits.map(|hit| hit as f64 / targets.max(1) as f64),
        admitted: admitted.map(|sum| sum / probes.len().max(1) as f64),
        route_time: Duration::ZERO,
        queries: probes.len().max(1),
    }
}

fn admitted_fraction(
    fixture: &RoutingFixture,
    selected: &[CommunityId],
    excluded: crate::MemoryId,
) -> f64 {
    let selected: HashSet<_> = selected.iter().copied().collect();
    let admitted = fixture
        .vectors
        .keys()
        .filter(|memory| **memory != excluded)
        .filter(|memory| match fixture.memberships.get(memory) {
            Some(community) => selected.contains(community),
            None => true,
        })
        .count();
    admitted as f64 / fixture.vectors.len().saturating_sub(1).max(1) as f64
}

fn target_count(probes: &[RoutingProbe]) -> usize {
    probes.iter().map(|probe| probe.targets.len()).sum()
}
