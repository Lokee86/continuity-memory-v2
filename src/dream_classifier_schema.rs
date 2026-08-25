use serde_json::{Value, json};

pub const DREAM_CLASSIFIER_SYSTEM_PROMPT: &str = r#"You classify the semantic relationship between exactly two durable Memories.

The pair is presented in canonical MemoryId order as A and B. A/B do NOT mean source/candidate, old/new, cause/effect, or processing order. Determine direction only from semantic evidence in the Memories.

Choose exactly one relation:
- none: no useful semantic relationship.
- topical: clearly about the same subject, but no narrower relation below applies.
- factual: one Memory supplies a fact, premise, or factual context used/asserted by the other.
- causal: one Memory explicitly causes, enables, prevents, or materially produces the state/event in the other.
- recurrent: distinct observations/occurrences of the same pattern, not semantically identical duplicates.
- duplicate_of: materially the same durable observation/state with no meaningful semantic difference.
- supersedes: one Memory explicitly corrects, replaces, invalidates, or becomes the operative version of the other.

Direction rules:
- none -> direction none.
- topical, recurrent, duplicate_of -> direction undirected.
- factual, causal, supersedes -> direction a_to_b or b_to_a according to meaning.

Source timestamps are chronology context only. They are not proof of causality or supersession. Existing Graph relations are supplemental context only and are not proof of the pair conclusion.

For every non-none conclusion, provide exactly two short verbatim evidence quotes: one copied from A title/content and one copied from B title/content. For none, evidence must be empty. Do not paraphrase evidence."#;

pub fn dream_classifier_schema() -> Value {
    json!({
        "type": "object",
        "additionalProperties": false,
        "required": ["relation", "direction", "evidence"],
        "properties": {
            "relation": {
                "type": "string",
                "enum": ["none", "topical", "factual", "causal", "recurrent", "duplicate_of", "supersedes"]
            },
            "direction": {
                "type": "string",
                "enum": ["none", "undirected", "a_to_b", "b_to_a"]
            },
            "evidence": {
                "type": "array",
                "maxItems": 2,
                "items": {
                    "type": "object",
                    "additionalProperties": false,
                    "required": ["side", "quote"],
                    "properties": {
                        "side": {"type": "string", "enum": ["a", "b"]},
                        "quote": {"type": "string", "minLength": 1, "maxLength": 500}
                    }
                }
            }
        }
    })
}
