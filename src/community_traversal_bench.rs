use crate::community_routing_bench_fixture::load_fixture;
use crate::community_traversal_bench_support::{TraversalPolicy, evaluate, policy_name};

const BUDGETS: [usize; 3] = [16, 32, 64];

#[test]
fn community_preferred_traversal_benchmark() {
    let fixture = load_fixture();
    println!(
        "policy,budget,queries,support_recall,nodes,edges,communities,boundary_crossings,tokens,us"
    );
    for budget in BUDGETS {
        for policy in [
            TraversalPolicy::Graph,
            TraversalPolicy::CommunityPreferred,
            TraversalPolicy::CommunityWithinDepth,
        ] {
            let totals = evaluate(&fixture, policy, budget);
            let queries = totals.queries.max(1) as f64;
            let recall = totals.support_hits as f64 / totals.support_total.max(1) as f64;
            println!(
                "{},{budget},{},{recall:.4},{:.2},{:.2},{:.2},{:.2},{:.1},{:.3}",
                policy_name(policy),
                totals.queries,
                totals.nodes as f64 / queries,
                totals.edges as f64 / queries,
                totals.communities as f64 / queries,
                totals.boundary_crossings as f64 / queries,
                totals.tokens as f64 / queries,
                totals.elapsed_ns as f64 / queries / 1000.0,
            );
        }
    }
}
