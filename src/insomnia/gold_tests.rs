use std::collections::HashSet;

use serde_json::Value;

#[test]
fn insomnia_gold_set_is_well_formed_and_balanced() {
    let root: Value = serde_json::from_str(include_str!("../../corpus/insomnia-gold-v1.json"))
        .expect("gold JSON must parse");
    assert_eq!(root["version"], "insomnia-gold-v1");

    let cases = root["cases"].as_array().expect("cases must be an array");
    assert_eq!(cases.len(), 40);

    let mut ids = HashSet::new();
    let mut sources = HashSet::new();
    let mut retain_count = 0;
    for case in cases {
        let id = required_string(case, "id");
        let episode = required_string(case, "episode_id");
        let source = required_string(case, "source_node_id");
        assert!(ids.insert(id), "duplicate gold case id: {id}");
        assert!(sources.insert(source), "duplicate gold source: {source}");
        assert_eq!(episode.len(), 64, "episode ids are SHA-256 hex");
        assert!(!required_string(case, "source_excerpt").is_empty());

        let retain = case["retain"].as_bool().expect("retain must be boolean");
        let receipt = required_string(case, "receipt_policy");
        assert!(matches!(receipt, "none" | "omit" | "strip_progress"));
        let source_policy = required_string(case, "content_source");
        assert!(matches!(
            source_policy,
            "required" | "forbidden" | "not_applicable"
        ));

        if retain {
            retain_count += 1;
            let authority = required_string(case, "authority_kind");
            assert!(matches!(
                authority,
                "direct" | "correction" | "adoption" | "retention"
            ));
            assert_ne!(source_policy, "not_applicable");
            assert!(!required_array(case, "acceptable_categories").is_empty());
            assert!(!required_array(case, "acceptable_types").is_empty());
            assert!(!required_string(case, "semantic_target").is_empty());
        } else {
            assert!(case["authority_kind"].is_null());
            assert_eq!(source_policy, "not_applicable");
            assert!(required_array(case, "acceptable_categories").is_empty());
            assert!(required_array(case, "acceptable_types").is_empty());
            assert!(case["semantic_target"].is_null());
        }
        required_array(case, "forbidden_content");
    }
    assert_eq!(retain_count, 32);
    assert_eq!(cases.len() - retain_count, 8);
}

fn required_string<'a>(value: &'a Value, key: &str) -> &'a str {
    value[key]
        .as_str()
        .unwrap_or_else(|| panic!("{key} must be a string"))
}

fn required_array<'a>(value: &'a Value, key: &str) -> &'a Vec<Value> {
    value[key]
        .as_array()
        .unwrap_or_else(|| panic!("{key} must be an array"))
}
