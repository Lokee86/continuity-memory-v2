use super::enrichment::enrich;
use super::extraction::{InsomniaCandidate, InsomniaRoutingMetadata};
use super::ownership::InsomniaOwnership;
use crate::{MemoryTextField, SimulatedGeneralEndpoint};
use serde_json::json;

#[test]
fn enrichment_maps_verbatim_mentions_to_durable_text_spans() {
    let mut candidates = vec![candidate()];
    let endpoint = SimulatedGeneralEndpoint::new(
        "metadata",
        vec![json!({
            "memories": {
                "claim-1": {
                    "entity_mentions": [
                        {"field":"content","text":"Sarah"},
                        {"field":"content","text":"the Vancouver office"},
                        {"field":"content","text":"Warlock"}
                    ],
                    "lexical_terms": ["Vancouver office", "Warlock"]
                }
            }
        })],
    );

    enrich(&endpoint, &mut candidates).unwrap();
    let routing = candidates[0].routing_metadata.as_ref().unwrap();
    assert_eq!(routing.lexical_terms, vec!["Vancouver office", "Warlock"]);
    assert_eq!(routing.entity_mentions.len(), 3);
    assert_eq!(routing.entity_mentions[0].field, MemoryTextField::Content);
    for mention in &routing.entity_mentions {
        let start = mention.start_byte as usize;
        let end = mention.end_byte as usize;
        assert_eq!(&candidates[0].content[start..end], mention.text);
    }
}

#[test]
fn enrichment_rejects_expansion_beyond_durable_mention_limit() {
    let mut candidate = candidate();
    candidate.content = std::iter::repeat_n("Sarah", 65)
        .collect::<Vec<_>>()
        .join(" ");
    let mut candidates = vec![candidate];
    let endpoint = SimulatedGeneralEndpoint::new(
        "metadata",
        vec![json!({
            "memories": {
                "claim-1": {
                    "entity_mentions": [{"field":"content","text":"Sarah"}],
                    "lexical_terms": []
                }
            }
        })],
    );

    let error = enrich(&endpoint, &mut candidates).unwrap_err();
    assert!(error.to_string().contains("bounded item limit"));
}

#[test]
fn enrichment_does_not_expand_word_mentions_inside_longer_identifiers() {
    let mut candidate = candidate();
    candidate.content = "Use `wgit` instead of `git`; damageResult wraps damage.".into();
    let mut candidates = vec![candidate];
    let endpoint = SimulatedGeneralEndpoint::new(
        "metadata",
        vec![json!({
            "memories": {
                "claim-1": {
                    "entity_mentions": [
                        {"field":"content","text":"git"},
                        {"field":"content","text":"damage"}
                    ],
                    "lexical_terms": []
                }
            }
        })],
    );

    enrich(&endpoint, &mut candidates).unwrap();
    let routing = candidates[0].routing_metadata.as_ref().unwrap();
    let mentions = routing
        .entity_mentions
        .iter()
        .map(|mention| (mention.text.as_str(), mention.start_byte, mention.end_byte))
        .collect::<Vec<_>>();
    assert_eq!(mentions, vec![("git", 23, 26), ("damage", 48, 54)]);
}

#[test]
fn enrichment_drops_only_embedded_word_occurrences() {
    let mut candidate = candidate();
    candidate.content = "damageResult is populated.".into();
    let mut candidates = vec![candidate];
    let endpoint = SimulatedGeneralEndpoint::new(
        "metadata",
        vec![json!({
            "memories": {
                "claim-1": {
                    "entity_mentions": [{"field":"content","text":"damage"}],
                    "lexical_terms": []
                }
            }
        })],
    );

    enrich(&endpoint, &mut candidates).unwrap();
    let routing = candidates[0].routing_metadata.as_ref().unwrap();
    assert!(routing.entity_mentions.is_empty());
}

#[test]
fn enrichment_normalizes_ascii_case_only_drift_to_source_text() {
    let mut candidate = candidate();
    candidate.title = "Diagnostic Backout".into();
    candidate.content = "The user prefers Git.".into();
    let mut candidates = vec![candidate];
    let endpoint = SimulatedGeneralEndpoint::new(
        "metadata",
        vec![json!({
            "memories": {
                "claim-1": {
                    "entity_mentions": [{"field":"content","text":"the user"}],
                    "lexical_terms": ["backout"]
                }
            }
        })],
    );

    enrich(&endpoint, &mut candidates).unwrap();
    let routing = candidates[0].routing_metadata.as_ref().unwrap();
    assert_eq!(routing.entity_mentions[0].text, "The user");
    assert_eq!(routing.lexical_terms, vec!["Backout"]);
}

#[test]
fn enrichment_keeps_most_specific_overlapping_mentions() {
    let mut candidate = candidate();
    candidate.content =
        "Windows Git and Git; Go uses Go-game-server; State wraps Game.State.".into();
    let mut candidates = vec![candidate];
    let endpoint = SimulatedGeneralEndpoint::new(
        "metadata",
        vec![json!({
            "memories": {
                "claim-1": {
                    "entity_mentions": [
                        {"field":"content","text":"Windows"},
                        {"field":"content","text":"Git"},
                        {"field":"content","text":"Windows Git"},
                        {"field":"content","text":"Go"},
                        {"field":"content","text":"Go-game-server"},
                        {"field":"content","text":"State"},
                        {"field":"content","text":"Game.State"}
                    ],
                    "lexical_terms": []
                }
            }
        })],
    );

    enrich(&endpoint, &mut candidates).unwrap();
    let routing = candidates[0].routing_metadata.as_ref().unwrap();
    let mentions = routing
        .entity_mentions
        .iter()
        .map(|mention| mention.text.as_str())
        .collect::<Vec<_>>();
    assert_eq!(
        mentions,
        vec![
            "Windows Git",
            "Git",
            "Go",
            "Go-game-server",
            "State",
            "Game.State"
        ]
    );
}

#[test]
fn enrichment_strips_inline_code_delimiters_from_entity_mentions() {
    let mut candidate = candidate();
    candidate.content = "Move `game.gd` into `networking/packets`.".into();
    let mut candidates = vec![candidate];
    let endpoint = SimulatedGeneralEndpoint::new(
        "metadata",
        vec![json!({
            "memories": {
                "claim-1": {
                    "entity_mentions": [
                        {"field":"content","text":"`game.gd`"},
                        {"field":"content","text":"`networking/packets`"}
                    ],
                    "lexical_terms": []
                }
            }
        })],
    );

    enrich(&endpoint, &mut candidates).unwrap();
    let routing = candidates[0].routing_metadata.as_ref().unwrap();
    let mentions = routing
        .entity_mentions
        .iter()
        .map(|mention| mention.text.as_str())
        .collect::<Vec<_>>();
    assert_eq!(mentions, vec!["game.gd", "networking/packets"]);
}

#[test]
fn enrichment_drops_invented_entity_names_but_keeps_valid_siblings() {
    let mut candidates = vec![candidate()];
    let endpoint = SimulatedGeneralEndpoint::new(
        "metadata",
        vec![json!({
            "memories": {
                "claim-1": {
                    "entity_mentions": [{"field":"content","text":"Sarah Chen"}],
                    "lexical_terms": ["Warlock"]
                }
            }
        })],
    );

    enrich(&endpoint, &mut candidates).unwrap();
    let routing = candidates[0].routing_metadata.as_ref().unwrap();
    assert!(routing.entity_mentions.is_empty());
    assert_eq!(routing.lexical_terms, vec!["Warlock"]);
}

#[test]
fn enrichment_drops_nonverbatim_lexical_terms_but_keeps_valid_mentions() {
    let mut candidates = vec![candidate()];
    let endpoint = SimulatedGeneralEndpoint::new(
        "metadata",
        vec![json!({
            "memories": {
                "claim-1": {
                    "entity_mentions": [{"field":"content","text":"Sarah"}],
                    "lexical_terms": ["Vancouver office", "read-only telemetry"]
                }
            }
        })],
    );

    enrich(&endpoint, &mut candidates).unwrap();
    let routing = candidates[0].routing_metadata.as_ref().unwrap();
    assert_eq!(routing.entity_mentions.len(), 1);
    assert_eq!(routing.entity_mentions[0].text, "Sarah");
    assert_eq!(routing.lexical_terms, vec!["Vancouver office"]);
}

fn candidate() -> InsomniaCandidate {
    InsomniaCandidate {
        key: "claim-1".into(),
        authority_kind: "direct".into(),
        category: "fact".into(),
        memory_type: "project".into(),
        temporal_status: "current".into(),
        ownership: InsomniaOwnership::Project,
        title: "Reliquary entity work".into(),
        content: "Sarah works at the Vancouver office with Warlock.".into(),
        source_node_id: "u1".into(),
        source_quote: "Sarah works at the Vancouver office with Warlock.".into(),
        authority_source_conversation_id: None,
        authority_source_node_id: None,
        authority_source_quote: None,
        grounding_source_conversation_id: None,
        grounding_source_node_id: None,
        grounding_source_quote: None,
        routing_metadata: None::<InsomniaRoutingMetadata>,
    }
}
