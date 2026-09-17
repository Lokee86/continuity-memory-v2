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
fn enrichment_rejects_only_embedded_word_occurrences() {
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

    let error = enrich(&endpoint, &mut candidates).unwrap_err();
    assert!(error.to_string().contains("boundary-compatible"));
}

#[test]
fn enrichment_rejects_invented_entity_names() {
    let mut candidates = vec![candidate()];
    let endpoint = SimulatedGeneralEndpoint::new(
        "metadata",
        vec![json!({
            "memories": {
                "claim-1": {
                    "entity_mentions": [{"field":"content","text":"Sarah Chen"}],
                    "lexical_terms": []
                }
            }
        })],
    );

    let error = enrich(&endpoint, &mut candidates).unwrap_err();
    assert!(
        error
            .to_string()
            .contains("not verbatim durable Memory text")
    );
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
