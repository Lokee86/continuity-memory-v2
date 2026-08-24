LEDGER_SCHEMA = {
    "type": "object",
    "additionalProperties": False,
    "properties": {
        "entries": {
            "type": "array",
            "maxItems": 128,
            "items": {
                "type": "object",
                "additionalProperties": False,
                "properties": {
                    "source_node_id": {"type": "string"},
                    "source_quote": {"type": "string"},
                    "disposition": {"type": "string", "enum": ["retain", "omit", "superseded"]},
                    "authority_kind": {"type": "string", "enum": ["direct", "correction", "adoption", "retention", "none"]},
                    "category": {"type": "string", "enum": ["fact", "preference", "decision", "instruction", "relationship", "constraint", "correction", "commitment", "none"]},
                    "type": {"type": "string", "enum": ["identity", "education", "employment", "location", "possession", "health", "finance", "schedule", "communication", "project", "process", "product", "relationship", "other", "none"]},
                    "lifecycle": {"type": "string", "enum": ["current", "future", "historical", "superseded", "none"]},
                    "proposition": {"type": "string"},
                    "authority_source_node_id": {"type": "string"},
                    "grounding_source_node_id": {"type": "string"},
                    "reason": {"type": "string"},
                },
                "required": [
                    "source_node_id", "source_quote", "disposition", "authority_kind",
                    "category", "type", "lifecycle", "proposition",
                    "authority_source_node_id", "grounding_source_node_id", "reason",
                ],
            },
        }
    },
    "required": ["entries"],
}

CANDIDATE_SCHEMA = {
    "type": "object",
    "additionalProperties": False,
    "properties": {
        "authority_kind": {"type": "string", "enum": ["direct", "correction", "adoption", "retention"]},
        "category": {"type": "string", "enum": ["fact", "preference", "decision", "instruction", "relationship", "constraint", "correction", "commitment"]},
        "type": {"type": "string", "enum": ["identity", "education", "employment", "location", "possession", "health", "finance", "schedule", "communication", "project", "process", "product", "relationship", "other"]},
        "title": {"type": "string"},
        "content": {"type": "string"},
        "source_node_id": {"type": "string"},
        "source_quote": {"type": "string"},
        "authority_source_conversation_id": {"type": "string"},
        "authority_source_node_id": {"type": "string"},
        "authority_source_quote": {"type": "string"},
        "grounding_source_conversation_id": {"type": "string"},
        "grounding_source_node_id": {"type": "string"},
        "grounding_source_quote": {"type": "string"},
    },
    "required": [
        "authority_kind", "category", "type", "title", "content",
        "source_node_id", "source_quote", "authority_source_conversation_id",
        "authority_source_node_id", "authority_source_quote",
        "grounding_source_conversation_id", "grounding_source_node_id",
        "grounding_source_quote",
    ],
}

SYNTHESIS_SCHEMA = {
    "type": "object",
    "additionalProperties": False,
    "properties": {
        "candidates": {"type": "array", "maxItems": 64, "items": CANDIDATE_SCHEMA}
    },
    "required": ["candidates"],
}

from pathlib import Path


def _rust_contract_prompt(name):
    contract = Path(__file__).resolve().parents[1] / "examples" / "insomnia_two_pass_sol" / "contract.rs"
    text = contract.read_text(encoding="utf-8")
    start_marker = f'pub const {name}: &str = r#"'
    start = text.index(start_marker) + len(start_marker)
    end = text.index('"#;', start)
    return text[start:end]


# Keep Ox/Nous experiments on the exact same semantic contract as the Rust/Sol harness.
LEDGER_PROMPT = _rust_contract_prompt("LEDGER_PROMPT")
SYNTHESIS_PROMPT = _rust_contract_prompt("SYNTHESIS_PROMPT")
