#[path = "ego_codec_scalar.rs"]
mod scalar;

use crate::{EgoAnchorId, EgoAnchorPriority, EgoError};
use scalar::{
    invalid, priority_byte, priority_from_byte, read_array, read_byte, read_string, read_u64,
    write_string, write_u64,
};
pub(crate) use scalar::{validate_anchor_text, validate_text};

const IDENTITY_MAGIC: [u8; 8] = *b"CVAEIDN1";
const PERSONALITY_MAGIC: [u8; 8] = *b"CVAEPER1";
const ANCHOR_MAGIC: [u8; 8] = *b"CVAEANC1";
const SYNTHESIS_MAGIC: [u8; 8] = *b"CVAESYN1";

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum EgoRecord {
    Identity {
        ego_version: u64,
        revision: u64,
        text: String,
    },
    Personality {
        ego_version: u64,
        revision: u64,
        source_memory_version: u64,
        text: String,
    },
    Anchor {
        ego_version: u64,
        id: EgoAnchorId,
        revision: u64,
        deleted: bool,
        priority: EgoAnchorPriority,
        text: String,
    },
    Synthesis {
        ego_version: u64,
        revision: u64,
        source_memory_version: u64,
        text: String,
    },
}

pub(crate) fn encode(record: &EgoRecord) -> Result<Vec<u8>, EgoError> {
    let mut out = Vec::new();
    match record {
        EgoRecord::Identity {
            ego_version,
            revision,
            text,
        } => {
            validate_text("identity", text)?;
            out.extend_from_slice(&IDENTITY_MAGIC);
            write_u64(&mut out, *ego_version);
            write_u64(&mut out, *revision);
            write_string(&mut out, text)?;
        }
        EgoRecord::Personality {
            ego_version,
            revision,
            source_memory_version,
            text,
        } => {
            validate_text("personality", text)?;
            out.extend_from_slice(&PERSONALITY_MAGIC);
            write_u64(&mut out, *ego_version);
            write_u64(&mut out, *revision);
            write_u64(&mut out, *source_memory_version);
            write_string(&mut out, text)?;
        }
        EgoRecord::Anchor {
            ego_version,
            id,
            revision,
            deleted,
            priority,
            text,
        } => {
            if !*deleted {
                validate_anchor_text(text)?;
            } else if !text.is_empty() {
                return Err(invalid("deleted anchor must have an empty body"));
            }
            out.extend_from_slice(&ANCHOR_MAGIC);
            write_u64(&mut out, *ego_version);
            out.extend_from_slice(&id.0);
            write_u64(&mut out, *revision);
            out.push(u8::from(*deleted));
            out.push(priority_byte(*priority));
            write_string(&mut out, text)?;
        }
        EgoRecord::Synthesis {
            ego_version,
            revision,
            source_memory_version,
            text,
        } => {
            validate_text("synthesis", text)?;
            out.extend_from_slice(&SYNTHESIS_MAGIC);
            write_u64(&mut out, *ego_version);
            write_u64(&mut out, *revision);
            write_u64(&mut out, *source_memory_version);
            write_string(&mut out, text)?;
        }
    }
    Ok(out)
}

pub(crate) fn decode(bytes: &[u8]) -> Result<Option<EgoRecord>, EgoError> {
    if bytes.len() < 8 {
        return Ok(None);
    }
    let mut cursor = 8;
    let record = if bytes[..8] == IDENTITY_MAGIC {
        let ego_version = read_u64(bytes, &mut cursor)?;
        let revision = read_u64(bytes, &mut cursor)?;
        let text = read_string(bytes, &mut cursor)?;
        validate_text("identity", &text)?;
        EgoRecord::Identity {
            ego_version,
            revision,
            text,
        }
    } else if bytes[..8] == PERSONALITY_MAGIC {
        let ego_version = read_u64(bytes, &mut cursor)?;
        let revision = read_u64(bytes, &mut cursor)?;
        let source_memory_version = read_u64(bytes, &mut cursor)?;
        let text = read_string(bytes, &mut cursor)?;
        validate_text("personality", &text)?;
        EgoRecord::Personality {
            ego_version,
            revision,
            source_memory_version,
            text,
        }
    } else if bytes[..8] == ANCHOR_MAGIC {
        let ego_version = read_u64(bytes, &mut cursor)?;
        let id = EgoAnchorId(read_array::<16>(bytes, &mut cursor)?);
        let revision = read_u64(bytes, &mut cursor)?;
        let deleted = match read_byte(bytes, &mut cursor)? {
            0 => false,
            1 => true,
            _ => return Err(invalid("anchor deletion flag is invalid")),
        };
        let priority = priority_from_byte(read_byte(bytes, &mut cursor)?)?;
        let text = read_string(bytes, &mut cursor)?;
        if deleted {
            if !text.is_empty() {
                return Err(invalid("deleted anchor must have an empty body"));
            }
        } else {
            validate_anchor_text(&text)?;
        }
        EgoRecord::Anchor {
            ego_version,
            id,
            revision,
            deleted,
            priority,
            text,
        }
    } else if bytes[..8] == SYNTHESIS_MAGIC {
        let ego_version = read_u64(bytes, &mut cursor)?;
        let revision = read_u64(bytes, &mut cursor)?;
        let source_memory_version = read_u64(bytes, &mut cursor)?;
        let text = read_string(bytes, &mut cursor)?;
        validate_text("synthesis", &text)?;
        EgoRecord::Synthesis {
            ego_version,
            revision,
            source_memory_version,
            text,
        }
    } else {
        return Ok(None);
    };
    if cursor != bytes.len() {
        return Err(invalid("record has trailing bytes"));
    }
    Ok(Some(record))
}

pub(crate) fn record_ego_version(record: &EgoRecord) -> u64 {
    match record {
        EgoRecord::Identity { ego_version, .. }
        | EgoRecord::Personality { ego_version, .. }
        | EgoRecord::Anchor { ego_version, .. }
        | EgoRecord::Synthesis { ego_version, .. } => *ego_version,
    }
}
