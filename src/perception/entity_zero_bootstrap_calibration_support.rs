use crate::{
    ConfiguredGeneralEndpoint, Cva, MemoryEntityMention, MemoryEntityMentionKey, MemoryId,
    MemoryRoutingMetadata, MemoryTextField, ModelProvider, ModelReasoningEffort, ModelSwitchboard,
    ReliquaryConfig,
};
use serde_json::Value;
use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
};

pub(crate) struct QueryCase {
    pub query_id: String,
    pub key: MemoryEntityMentionKey,
    pub expected_decision: String,
    pub expected_entity: Option<String>,
    pub sort_key: (i64, [u8; 32], u32),
}

pub(crate) fn live_resolver_endpoint() -> ConfiguredGeneralEndpoint {
    let config = ReliquaryConfig::open(Path::new("reliquary.cfg")).unwrap();
    let mut models = config.models.clone();
    let route = models
        .entity_resolution
        .as_mut()
        .expect("calibration config requires entity_resolution route");
    assert_eq!(route.provider, ModelProvider::OpenAiCodex);
    route.model = "gpt-5.6-sol".into();
    route.reasoning_effort = Some(ModelReasoningEffort::Low);
    ConfiguredGeneralEndpoint::from_entity_resolution_switchboard(
        &ModelSwitchboard::new(models, config.credentials.clone()).unwrap(),
    )
    .unwrap()
}

pub(crate) fn copy_frozen_rel() -> PathBuf {
    let source = Path::new("fixtures/local/frozen/chatgpt-first28d-2026-08-30/project.prj.rel");
    let path = std::env::temp_dir().join(format!(
        "reliquary-zero-entity-bootstrap-{}.rel",
        uuid::Uuid::new_v4()
    ));
    fs::copy(source, &path).unwrap();
    path
}

pub(crate) fn prepare_real_queries(rel: &mut Cva) -> Vec<QueryCase> {
    let root = Path::new("fixtures/local/calibration/entity-store-resolution-frozen-rel-v1");
    let queries = load_jsonl(&root.join("queries.jsonl"));
    let gold: Value =
        serde_json::from_str(&fs::read_to_string(root.join("gold.json")).unwrap()).unwrap();

    let mut mentions = HashMap::<MemoryId, Vec<MemoryEntityMention>>::new();
    let mut cases = Vec::new();
    for query in queries {
        let query_id = query["memory_id"].as_str().unwrap().to_owned();
        let source_id = parse_memory_id(query["source_memory_id"].as_str().unwrap());
        let mention = mention_from_fixture(&query["mentions"][0]);
        let memory = rel.memory(source_id).unwrap();
        assert_fixture_span(&memory, &mention);
        let key = MemoryEntityMentionKey::new(source_id, &mention);
        mentions.entry(source_id).or_default().push(mention);
        let expected = &gold[&query_id];
        cases.push(QueryCase {
            query_id,
            key,
            expected_decision: expected["expected_decision"].as_str().unwrap().to_owned(),
            expected_entity: expected["expected_entity_id"].as_str().map(str::to_owned),
            sort_key: (
                memory.source_time_ns.unwrap_or(memory.created_at_ns),
                source_id.0,
                key.start_byte,
            ),
        });
    }

    for (memory_id, mut values) in mentions {
        values.sort_by_key(|mention| (mention.field.tag(), mention.start_byte, mention.end_byte));
        values.dedup_by(|left, right| {
            left.field == right.field
                && left.start_byte == right.start_byte
                && left.end_byte == right.end_byte
        });
        let body_id = rel.memory_body_id(memory_id).unwrap();
        rel.memories
            .put_routing_metadata(
                &mut rel.container,
                MemoryRoutingMetadata {
                    memory_id,
                    body_id,
                    entity_mentions: values,
                },
            )
            .unwrap();
    }
    cases.sort_by_key(|case| case.sort_key);
    cases
}

fn load_jsonl(path: &Path) -> Vec<Value> {
    fs::read_to_string(path)
        .unwrap()
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| serde_json::from_str(line).unwrap())
        .collect()
}

fn parse_memory_id(value: &str) -> MemoryId {
    assert_eq!(value.len(), 64);
    let mut bytes = [0_u8; 32];
    for (index, byte) in bytes.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&value[index * 2..index * 2 + 2], 16).unwrap();
    }
    MemoryId(bytes)
}

fn mention_from_fixture(value: &Value) -> MemoryEntityMention {
    MemoryEntityMention {
        field: match value["field"].as_str().unwrap() {
            "title" => MemoryTextField::Title,
            "content" => MemoryTextField::Content,
            other => panic!("unknown mention field {other}"),
        },
        start_byte: value["start_byte"].as_u64().unwrap() as u32,
        end_byte: value["end_byte"].as_u64().unwrap() as u32,
        text: value["text"].as_str().unwrap().to_owned(),
    }
}

fn assert_fixture_span(memory: &crate::Memory, mention: &MemoryEntityMention) {
    let source = match mention.field {
        MemoryTextField::Title => &memory.title,
        MemoryTextField::Content => &memory.content,
    };
    assert_eq!(
        source.get(mention.start_byte as usize..mention.end_byte as usize),
        Some(mention.text.as_str())
    );
}
