use reliquary_memory::{
    MemoryBodyId, MemoryEntityMention, MemoryId, MemoryRoutingMetadata, MemoryTextField,
};
use serde_json::Value;
use std::error::Error;
use std::fs;

#[derive(Clone)]
pub struct ExtractionRow {
    pub owner: Owner,
    pub metadata: MemoryRoutingMetadata,
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub enum Owner {
    Rel,
    Phy,
}

pub fn load_rows(path: &str) -> Result<Vec<ExtractionRow>, Box<dyn Error>> {
    fs::read_to_string(path)?
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| parse_row(&serde_json::from_str::<Value>(line)?))
        .collect()
}

fn parse_row(row: &Value) -> Result<ExtractionRow, Box<dyn Error>> {
    if !row["error"].is_null() {
        return Err(format!("extraction row contains error: {}", row["error"]).into());
    }
    let owner = match row["owner_kind"].as_str() {
        Some("rel") => Owner::Rel,
        Some("phy") => Owner::Phy,
        _ => return Err("invalid owner_kind".into()),
    };
    let memory_id = MemoryId(hex32(required(row, "memory_id")?)?);
    let body_id = MemoryBodyId(hex32(required(row, "body_id")?)?);
    let mentions = row["actual_entity_mentions"]
        .as_array()
        .ok_or("actual_entity_mentions is not an array")?
        .iter()
        .map(parse_mention)
        .collect::<Result<Vec<_>, _>>()?;
    Ok(ExtractionRow {
        owner,
        metadata: MemoryRoutingMetadata {
            memory_id,
            body_id,
            entity_mentions: mentions,
        },
    })
}

fn parse_mention(value: &Value) -> Result<MemoryEntityMention, Box<dyn Error>> {
    let field = match value["field"].as_str() {
        Some("title") => MemoryTextField::Title,
        Some("content") => MemoryTextField::Content,
        _ => return Err("invalid mention field".into()),
    };
    Ok(MemoryEntityMention {
        field,
        start_byte: value["start_byte"].as_u64().ok_or("missing start_byte")? as u32,
        end_byte: value["end_byte"].as_u64().ok_or("missing end_byte")? as u32,
        text: required(value, "text")?.to_owned(),
    })
}

fn required<'a>(value: &'a Value, key: &str) -> Result<&'a str, Box<dyn Error>> {
    value[key]
        .as_str()
        .ok_or_else(|| format!("missing string field {key}").into())
}

pub fn hex32(value: &str) -> Result<[u8; 32], Box<dyn Error>> {
    if value.len() != 64 {
        return Err(format!("expected 64 hex chars, got {}", value.len()).into());
    }
    let mut out = [0_u8; 32];
    for (index, byte) in out.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&value[index * 2..index * 2 + 2], 16)?;
    }
    Ok(out)
}
