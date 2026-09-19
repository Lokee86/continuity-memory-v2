use crate::MAX_COMMUNITY_SEMANTIC_NAME_BYTES;
use serde_json::{Value, json};

pub const DREAM_COMMUNITY_NAMING_SYSTEM_PROMPT: &str = r#"You name one derived Memory-Web community from representative Memories selected from its semantic sub-centroids.

Return one concise semantic name that describes the shared subject of the representative Memories.

Rules:
- Use a concrete noun phrase, normally 2-6 words.
- Name the subject, not the fact that these are memories or a community.
- Prefer specific domain language present in the evidence.
- Do not invent entities, projects, people, or themes absent from the evidence.
- Avoid generic labels such as Miscellaneous, General, Other, Notes, or Memories.
- Do not add explanation, punctuation-only decoration, quotation marks, or a trailing period.
- Output only the schema fields."#;

pub fn dream_community_naming_schema() -> Value {
    json!({
        "type": "object",
        "additionalProperties": false,
        "required": ["name"],
        "properties": {
            "name": {
                "type": "string",
                "minLength": 1,
                "maxLength": MAX_COMMUNITY_SEMANTIC_NAME_BYTES
            }
        }
    })
}
