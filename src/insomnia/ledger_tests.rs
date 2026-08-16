use crate::{
    EpisodeBoundary, EpisodeConfig, EpisodeOrigin, InsomniaExtractor, SimulatedGeneralEndpoint,
};
use serde_json::json;

fn episode(turns: &[(&str, Option<&str>, &str, i64, &str)]) -> (crate::Cva, crate::Episode) {
    let path = std::env::temp_dir().join(format!(
        "continuity-ledger-test-{}.cva",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let mut cva = crate::Cva::create(path).unwrap();
    for (id, parent, role, time, content) in turns {
        cva.append_node(
            (*id).into(),
            "c1".into(),
            parent.map(str::to_owned),
            (*role).into(),
            *time,
            content,
        )
        .unwrap();
    }
    let leaf = turns.last().unwrap().0;
    let episode = cva
        .materialize_path_episodes(
            "c1",
            leaf,
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

fn omitted(quote: &str) -> serde_json::Value {
    json!({
        "source_quote": quote,
        "disposition": "omit",
        "authority_kind": "none",
        "category": "none",
        "type": "none",
        "lifecycle": "non_authoritative",
        "proposition": "",
        "reason": "not durable authority",
        "authority_source_conversation_id": "",
        "authority_source_node_id": "",
        "authority_source_quote": "",
        "grounding_source_conversation_id": "",
        "grounding_source_node_id": "",
        "grounding_source_quote": ""
    })
}

fn retained(quote: &str, authority: &str, category: &str, proposition: &str) -> serde_json::Value {
    json!({
        "source_quote": quote,
        "disposition": "retain",
        "authority_kind": authority,
        "category": category,
        "type": "project",
        "lifecycle": "current",
        "proposition": proposition,
        "reason": "durable state",
        "authority_source_conversation_id": "",
        "authority_source_node_id": "",
        "authority_source_quote": "",
        "grounding_source_conversation_id": "",
        "grounding_source_node_id": "",
        "grounding_source_quote": ""
    })
}

#[test]
fn ledger_preserves_short_assistant_adoption() {
    let (mut cva, episode) = episode(&[
        ("u0", None, "user", 10, "How should C and C++ share this?"),
        (
            "a0",
            Some("u0"),
            "assistant",
            20,
            "Use one shared C-family adapter with separate parsing where needed.",
        ),
        ("u1", Some("a0"), "user", 30, "Fuckit, add that"),
    ]);
    let turns = cva.episode_turns(episode.id).unwrap();
    let mut adoption = retained(
        "Fuckit, add that",
        "adoption",
        "decision",
        "Use one shared C-family adapter with separate C/C++ parsing where needed.",
    );
    adoption["authority_source_conversation_id"] = json!("c1");
    adoption["authority_source_node_id"] = json!("a0");
    adoption["authority_source_quote"] =
        json!("Use one shared C-family adapter with separate parsing where needed.");
    let extractor = InsomniaExtractor::new_ledger(SimulatedGeneralEndpoint::new(
        "sol-high",
        vec![
            json!({
                "turns": [
                    {"source_node_id":"u0","units":[omitted("How should C and C++ share this?")]},
                    {"source_node_id":"u1","units":[adoption]}
                ],
                "evidence_requests": []
            }),
            json!({"memories":[{
                "covered_ledger_ids":["u1:0"],
                "category":"decision",
                "title":"Shared C-family adapter",
                "content":"Use one shared C-family adapter with separate C/C++ parsing where needed."
            }]}),
        ],
    ));
    let extraction = extractor.extract(&episode, &turns).unwrap();
    assert_eq!(extraction.contract_version, "v2-9-ledger");
    assert_eq!(extraction.candidates.len(), 1);
    assert_eq!(extraction.candidates[0].authority_kind, "adoption");
    assert_eq!(
        extraction.candidates[0].authority_source_node_id.as_deref(),
        Some("a0")
    );
}

#[test]
fn ledger_splits_execution_receipt_from_durable_status() {
    let text = "Up to prompt 72 completed and everything seems to be working OK so far. Some visual bugs remain.";
    let (mut cva, episode) = episode(&[("u0", None, "user", 10, text)]);
    let turns = cva.episode_turns(episode.id).unwrap();
    let mut state = retained(
        "everything seems to be working OK so far. Some visual bugs remain.",
        "direct",
        "fact",
        "The implementation is generally working, with visual bugs remaining.",
    );
    state["lifecycle"] = json!("current");
    let extractor = InsomniaExtractor::new_ledger(SimulatedGeneralEndpoint::new(
        "sol-high",
        vec![
            json!({
                "turns":[{"source_node_id":"u0","units":[
                    omitted("Up to prompt 72 completed"), state
                ]}],
                "evidence_requests":[]
            }),
            json!({"memories":[{
                "covered_ledger_ids":["u0:1"],
                "category":"fact",
                "title":"Current implementation status",
                "content":"The implementation is generally working, with visual bugs remaining."
            }]}),
        ],
    ));
    let extraction = extractor.extract(&episode, &turns).unwrap();
    assert_eq!(extraction.candidates.len(), 1);
    assert!(!extraction.candidates[0].content.contains("72"));
}

#[test]
fn pure_progress_receipt_ends_after_disposition_pass() {
    let (mut cva, episode) = episode(&[(
        "u0",
        None,
        "user",
        10,
        "I've completed through 41 on that list I just gave you.",
    )]);
    let turns = cva.episode_turns(episode.id).unwrap();
    let extractor = InsomniaExtractor::new_ledger(SimulatedGeneralEndpoint::new(
        "sol-high",
        vec![json!({
            "turns":[{"source_node_id":"u0","units":[
                omitted("I've completed through 41 on that list I just gave you.")
            ]}],
            "evidence_requests":[]
        })],
    ));
    let extraction = extractor.extract(&episode, &turns).unwrap();
    assert!(extraction.candidates.is_empty());
}
