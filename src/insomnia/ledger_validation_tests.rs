use crate::{
    EpisodeBoundary, EpisodeConfig, EpisodeOrigin, InsomniaExtractionError, InsomniaExtractor,
    SimulatedGeneralEndpoint,
};
use serde_json::json;

fn one_turn(content: &str) -> (crate::Cva, crate::Episode) {
    let path = std::env::temp_dir().join(format!(
        "continuity-ledger-validation-{}.cva",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let mut cva = crate::Cva::create(path).unwrap();
    cva.append_node("u0".into(), "c1".into(), None, "user".into(), 10, content)
        .unwrap();
    let episode = cva
        .materialize_path_episodes(
            "c1",
            "u0",
            EpisodeConfig::default(),
            EpisodeOrigin::Live,
            Some((EpisodeBoundary::Inactivity, 100)),
        )
        .unwrap()
        .created
        .into_iter()
        .next()
        .unwrap();
    (cva, episode)
}

fn retained_unit(quote: &str, category: &str, proposition: &str) -> serde_json::Value {
    json!({
        "source_quote": quote,
        "disposition": "retain",
        "authority_kind": "direct",
        "category": category,
        "type": "project",
        "lifecycle": "current",
        "proposition": proposition,
        "reason": "durable assertion",
        "authority_source_conversation_id": "",
        "authority_source_node_id": "",
        "authority_source_quote": "",
        "grounding_source_conversation_id": "",
        "grounding_source_node_id": "",
        "grounding_source_quote": ""
    })
}

#[test]
fn asserted_confirmation_tag_survives_ledger_pipeline() {
    let quote = "collision would have to be handled on the server to keep it authoritative right?";
    let (mut cva, episode) = one_turn(quote);
    let turns = cva.episode_turns(episode.id).unwrap();
    let extractor = InsomniaExtractor::new_ledger(SimulatedGeneralEndpoint::new(
        "sol-high",
        vec![
            json!({
                "turns":[{"source_node_id":"u0","units":[
                    retained_unit(quote, "constraint", "Collision handling must remain server-authoritative.")
                ]}],
                "evidence_requests":[]
            }),
            json!({"memories":[{
                "covered_ledger_ids":["u0:0"],
                "category":"constraint",
                "title":"Server-authoritative collision",
                "content":"Collision handling must remain server-authoritative."
            }]}),
        ],
    ));
    let extraction = extractor.extract(&episode, &turns).unwrap();
    assert_eq!(extraction.candidates.len(), 1);
}

#[test]
fn synthesis_can_consolidate_compatible_units_without_dropping_coverage() {
    let text = "Use hue values for remote identity. Derive indicator colors from the same hues.";
    let (mut cva, episode) = one_turn(text);
    let turns = cva.episode_turns(episode.id).unwrap();
    let extractor = InsomniaExtractor::new_ledger(SimulatedGeneralEndpoint::new(
        "sol-high",
        vec![
            json!({
                "turns":[{"source_node_id":"u0","units":[
                    retained_unit("Use hue values for remote identity.", "decision", "Use hue values for remote identity."),
                    retained_unit("Derive indicator colors from the same hues.", "constraint", "Derive indicator colors from the same remote identity hues.")
                ]}],
                "evidence_requests":[]
            }),
            json!({"memories":[{
                "covered_ledger_ids":["u0:0","u0:1"],
                "category":"decision",
                "title":"Hue-based player identity",
                "content":"Use hue values for remote identity and derive indicator colors from those same hues."
            }]}),
        ],
    ));
    let extraction = extractor.extract(&episode, &turns).unwrap();
    assert_eq!(extraction.candidates.len(), 1);
    assert_eq!(extraction.candidates[0].source_quote, text);
}

#[test]
fn synthesis_cannot_drop_a_retained_unit() {
    let quote = "Custom room codes are not working right now.";
    let (mut cva, episode) = one_turn(quote);
    let turns = cva.episode_turns(episode.id).unwrap();
    let extractor = InsomniaExtractor::new_ledger(SimulatedGeneralEndpoint::new(
        "sol-high",
        vec![
            json!({
                "turns":[{"source_node_id":"u0","units":[
                    retained_unit(quote, "fact", "Custom room codes are currently unsupported.")
                ]}],
                "evidence_requests":[]
            }),
            json!({"memories":[]}),
        ],
    ));
    let error = extractor.extract(&episode, &turns).unwrap_err();
    assert!(matches!(error, InsomniaExtractionError::InvalidOutput(_)));
    assert!(error.to_string().contains("omitted retained ledger ids"));
}
