use crate::{
    DreamCandidateConfig, DreamProcessor, DreamVerificationPolicy, MemoryDraft, Phylactery,
    SimulatedEmbeddingEndpoint, SimulatedGeneralEndpoint, VectorNormalization,
};
use serde_json::json;
use std::fs;
use std::path::PathBuf;

fn test_path(name: &str) -> PathBuf {
    let unique = uuid::Uuid::new_v4();
    let dir = std::env::temp_dir().join(format!("reliquary-phy-dream-{unique}"));
    fs::create_dir_all(&dir).unwrap();
    dir.join(name)
}

fn draft(
    mutation_id: &str,
    category: &str,
    memory_type: &str,
    lifecycle: &str,
    content: &str,
    source_time_ns: i64,
) -> MemoryDraft {
    MemoryDraft {
        category: category.into(),
        memory_type: memory_type.into(),
        authority_kind: "direct".into(),
        title: mutation_id.into(),
        content: content.into(),
        scope: "user".into(),
        lifecycle_state: lifecycle.into(),
        archived: false,
        superseded_by: None,
        parent_id: None,
        source_node_id: None,
        content_source_conversation_id: None,
        content_source_node_id: None,
        grounding_source_conversation_id: None,
        grounding_source_node_id: None,
        source_episode_id: None,
        source_time_ns: Some(source_time_ns),
        mutation_id: mutation_id.into(),
        created_at_ns: source_time_ns.saturating_add(1_000),
        updated_at_ns: source_time_ns.saturating_add(1_000),
    }
}

fn vectorize(phy: &mut Phylactery) -> crate::CompatibilityProfileId {
    let endpoint = SimulatedEmbeddingEndpoint::new(8, VectorNormalization::L2, 17);
    let profile = phy.establish_compatibility_profile(&endpoint).unwrap();
    phy.build_missing_memory_vectors(profile.id, &endpoint)
        .unwrap();
    profile.id
}

#[test]
fn phy_dream_candidates_and_temporal_analysis_need_no_archive() {
    let mut phy = Phylactery::create(test_path("candidate.phy")).unwrap();
    let (source, _) = phy
        .publish_memory(
            None,
            0,
            draft(
                "source",
                "fact",
                "identity",
                "extracted",
                "Inspection is tomorrow.",
                1_787_572_800_000_000_000,
            ),
        )
        .unwrap();
    let (candidate, _) = phy
        .publish_memory(
            None,
            0,
            draft(
                "candidate",
                "fact",
                "identity",
                "knowledge",
                "Inspection scheduled for 2026-08-25.",
                1,
            ),
        )
        .unwrap();
    let profile = vectorize(&mut phy);

    let set = phy
        .dream_candidates(
            profile,
            source.id,
            DreamCandidateConfig {
                limit: 1,
                semantic_limit: 0,
                prior_semantic_quota: 0,
                lexical_limit: 0,
                temporal_limit: 1,
            },
        )
        .unwrap();

    assert_eq!(set.source.source_timestamp_ns, source.source_time_ns);
    assert_eq!(set.candidates[0].context.memory.id, candidate.id);
    assert!(!set.candidates[0].temporal_matches.is_empty());
    assert_eq!(
        phy.dream_temporal_analysis(source.id)
            .unwrap()
            .source_timestamp_ns,
        source.source_time_ns
    );
}

#[test]
fn phy_dream_processor_publishes_duplicate_chain_and_archives_only_in_phy() {
    let path = test_path("duplicate.phy");
    let mut phy = Phylactery::create(&path).unwrap();
    let (representative, _) = phy
        .publish_memory(
            None,
            0,
            draft(
                "representative",
                "fact",
                "identity",
                "knowledge",
                "Same memory.",
                100,
            ),
        )
        .unwrap();
    let (source, _) = phy
        .publish_memory(
            None,
            0,
            draft(
                "source-duplicate",
                "fact",
                "identity",
                "extracted",
                "Same memory.",
                110,
            ),
        )
        .unwrap();
    let profile = vectorize(&mut phy);
    let processor = DreamProcessor::new(
        SimulatedGeneralEndpoint::new(
            "classifier",
            vec![json!({
                "relation": "duplicate_of",
                "direction": "undirected",
                "evidence": [
                    {"side": "a", "quote": "Same memory."},
                    {"side": "b", "quote": "Same memory."}
                ]
            })],
        ),
        SimulatedGeneralEndpoint::new(
            "verifier",
            vec![json!({
                "relation_supported": "yes",
                "direction_supported": "yes",
                "evidence_supported": "yes"
            })],
        ),
    );

    let result = processor
        .process_phylactery_memory(
            &mut phy,
            profile,
            source.id,
            DreamCandidateConfig::default(),
            DreamVerificationPolicy::default(),
        )
        .unwrap();

    assert_eq!(result.candidate_count, 1);
    assert!(result.source.archived);
    assert_eq!(phy.graph_relations().len(), 1);
    assert!(!phy.memory(representative.id).unwrap().archived);
    phy.sync().unwrap();
    drop(phy);
    let reopened = Phylactery::open(path).unwrap();
    assert_eq!(reopened.graph_relations().len(), 1);
}

#[test]
fn phy_dream_frontier_processes_multiple_memories_with_one_inference_bound() {
    let mut phy = Phylactery::create(test_path("frontier.phy")).unwrap();
    let (first, _) = phy
        .publish_memory(
            None,
            0,
            draft(
                "frontier-first",
                "fact",
                "identity",
                "extracted",
                "First frontier memory.",
                100,
            ),
        )
        .unwrap();
    let (second, _) = phy
        .publish_memory(
            None,
            0,
            draft(
                "frontier-second",
                "fact",
                "identity",
                "extracted",
                "Second frontier memory.",
                110,
            ),
        )
        .unwrap();
    let profile = vectorize(&mut phy);
    let processor = DreamProcessor::new(
        SimulatedGeneralEndpoint::new(
            "classifier",
            vec![
                json!({"relation": "none", "direction": "none", "evidence": []}),
                json!({"relation": "none", "direction": "none", "evidence": []}),
            ],
        ),
        SimulatedGeneralEndpoint::new("verifier", vec![]),
    );

    let outcomes = processor
        .process_phylactery_memories_with_concurrency(
            &mut phy,
            profile,
            &[first.id, second.id],
            DreamCandidateConfig {
                limit: 1,
                semantic_limit: 1,
                prior_semantic_quota: 0,
                lexical_limit: 0,
                temporal_limit: 0,
            },
            DreamVerificationPolicy::default(),
            2,
            2,
        )
        .unwrap();

    assert_eq!(outcomes.len(), 2);
    assert!(outcomes.iter().all(|outcome| outcome.result.is_ok()));
    assert_eq!(phy.memory(first.id).unwrap().lifecycle_state, "knowledge");
    assert_eq!(phy.memory(second.id).unwrap().lifecycle_state, "knowledge");
}

#[test]
fn phy_lifecycle_does_not_invent_provenance_based_corroboration() {
    let mut phy = Phylactery::create(test_path("lifecycle.phy")).unwrap();
    let (preference, _) = phy
        .publish_memory(
            None,
            0,
            draft(
                "preference",
                "preference",
                "user",
                "extracted",
                "Prefer concise answers.",
                100,
            ),
        )
        .unwrap();
    let (fact, _) = phy
        .publish_memory(
            None,
            0,
            draft(
                "fact",
                "fact",
                "identity",
                "extracted",
                "The client is Acme.",
                110,
            ),
        )
        .unwrap();

    let preference_result = phy.reconcile_dream_lifecycle(preference.id).unwrap();
    let fact_result = phy.reconcile_dream_lifecycle(fact.id).unwrap();

    assert_eq!(preference_result.source.lifecycle_state, "canonical");
    assert_eq!(fact_result.source.lifecycle_state, "knowledge");
    assert!(fact_result.canonicalized.is_empty());
}
