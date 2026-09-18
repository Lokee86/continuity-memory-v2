use crate::entity_zero_bootstrap_calibration_support::{
    copy_frozen_rel, live_resolver_endpoint, prepare_real_queries,
};
use crate::{Cva, EntityCandidateConfig, EntityResolutionDecision, EntityResolver};
use std::collections::HashMap;

#[test]
#[ignore = "live Sol-low zero-Entity bootstrap over frozen real REL"]
fn frozen_rel_zero_entity_bootstrap_converges_without_preconstructed_entities() {
    let path = copy_frozen_rel();
    let mut rel = Cva::open(&path).unwrap();
    assert_eq!(rel.entity_stats().entities, 0);
    assert_eq!(
        rel.memory_ids()
            .iter()
            .filter(|id| rel.memory_routing_metadata(**id).is_some())
            .count(),
        0
    );

    let cases = prepare_real_queries(&mut rel);
    assert_eq!(cases.len(), 41);

    let resolver = EntityResolver::new(live_resolver_endpoint());
    let config = EntityCandidateConfig::default();
    let mut fixture_to_entity = HashMap::new();
    let mut created_identities = 0usize;
    let mut resolved_repeats = 0usize;
    let mut standalone_create = 0usize;
    let mut unresolved = 0usize;
    let mut reject = 0usize;

    for (index, case) in cases.iter().enumerate() {
        let outcome = rel
            .resolve_entity_mention(&resolver, case.key, config, 1_000 + index as i64)
            .unwrap();

        match case.expected_decision.as_str() {
            "resolve_existing" => {
                let fixture_id = case.expected_entity.as_ref().unwrap();
                match fixture_to_entity.get(fixture_id).copied() {
                    None => {
                        assert_eq!(
                            outcome.decision,
                            EntityResolutionDecision::CreateNew,
                            "first organic occurrence of {} / {} did not create",
                            fixture_id,
                            case.query_id
                        );
                        fixture_to_entity.insert(fixture_id.clone(), outcome.entity_id.unwrap());
                        created_identities += 1;
                    }
                    Some(expected) => {
                        assert_eq!(
                            outcome.decision,
                            EntityResolutionDecision::ResolveExisting(expected),
                            "repeat occurrence resolved incorrectly for {} / {}",
                            fixture_id,
                            case.query_id
                        );
                        resolved_repeats += 1;
                    }
                }
            }
            "create_new" => {
                assert_eq!(
                    outcome.decision,
                    EntityResolutionDecision::CreateNew,
                    "gold new identity did not split for {}",
                    case.query_id
                );
                standalone_create += 1;
            }
            "unresolved" => {
                assert_eq!(
                    outcome.decision,
                    EntityResolutionDecision::Unresolved,
                    "gold ambiguous query resolved unsafely for {}",
                    case.query_id
                );
                unresolved += 1;
            }
            "reject" => {
                assert_eq!(
                    outcome.decision,
                    EntityResolutionDecision::Reject,
                    "gold reject query was retained for {}",
                    case.query_id
                );
                reject += 1;
            }
            other => panic!("unknown expected decision {other}"),
        }
    }

    assert_eq!(fixture_to_entity.len(), 24);
    assert_eq!(created_identities, 24);
    assert_eq!(resolved_repeats, 13);
    assert_eq!(standalone_create, 1);
    assert_eq!(unresolved, 1);
    assert_eq!(reject, 2);
    assert_eq!(rel.entity_stats().entities, 25);

    rel.sync().unwrap();
    drop(rel);
    let reopened = Cva::open(&path).unwrap();
    assert_eq!(reopened.entity_stats().entities, 25);
    for case in &cases {
        assert!(
            reopened.entity_resolution(case.key).is_some(),
            "missing durable resolution for {}",
            case.query_id
        );
    }

    println!(
        "real zero-Entity bootstrap: queries=41 entities=25 \
         24 first-occurrence creates / 13 repeat resolves / \
         1 distinct create / 1 unresolved / 2 reject"
    );
    let _ = std::fs::remove_file(path);
}
