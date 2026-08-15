use crate::ConfigError;
use std::collections::BTreeMap;

pub(crate) const CONFIG_MAGIC: &[u8; 8] = b"CVCFG\0\r\n";
pub(crate) const CONFIG_MAJOR: u16 = 1;
pub(crate) const CONFIG_MINOR: u16 = 0;
const HEADER_LEN: usize = 16;
const OBJECT_HEADER_LEN: usize = 16;
const MAX_OBJECTS: usize = 4096;
const MAX_PAYLOAD_LEN: usize = 16 * 1024 * 1024;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct RawConfigObject {
    pub schema: u16,
    pub flags: u32,
    pub payload: Vec<u8>,
}

pub(crate) fn encode_config(
    objects: &BTreeMap<String, RawConfigObject>,
) -> Result<Vec<u8>, ConfigError> {
    if objects.len() > MAX_OBJECTS {
        return Err(ConfigError::InvalidObject);
    }
    let mut bytes = Vec::with_capacity(HEADER_LEN + objects.len() * 32);
    bytes.extend_from_slice(CONFIG_MAGIC);
    bytes.extend_from_slice(&CONFIG_MAJOR.to_le_bytes());
    bytes.extend_from_slice(&CONFIG_MINOR.to_le_bytes());
    bytes.extend_from_slice(&(objects.len() as u32).to_le_bytes());
    for (key, object) in objects {
        let key_bytes = key.as_bytes();
        let key_len = u16::try_from(key_bytes.len()).map_err(|_| ConfigError::InvalidObject)?;
        let payload_len =
            u32::try_from(object.payload.len()).map_err(|_| ConfigError::InvalidObject)?;
        if object.payload.len() > MAX_PAYLOAD_LEN {
            return Err(ConfigError::InvalidObject);
        }
        bytes.extend_from_slice(&key_len.to_le_bytes());
        bytes.extend_from_slice(&object.schema.to_le_bytes());
        bytes.extend_from_slice(&object.flags.to_le_bytes());
        bytes.extend_from_slice(&payload_len.to_le_bytes());
        bytes.extend_from_slice(&0_u32.to_le_bytes());
        bytes.extend_from_slice(key_bytes);
        bytes.extend_from_slice(&object.payload);
    }
    Ok(bytes)
}

pub(crate) fn decode_config(
    bytes: &[u8],
) -> Result<BTreeMap<String, RawConfigObject>, ConfigError> {
    if bytes.len() < HEADER_LEN {
        return Err(ConfigError::Truncated);
    }
    if &bytes[..8] != CONFIG_MAGIC {
        return Err(ConfigError::InvalidMagic);
    }
    let major = u16::from_le_bytes(bytes[8..10].try_into().unwrap());
    let minor = u16::from_le_bytes(bytes[10..12].try_into().unwrap());
    if major != CONFIG_MAJOR || minor != CONFIG_MINOR {
        return Err(ConfigError::UnsupportedVersion { major, minor });
    }
    let count = u32::from_le_bytes(bytes[12..16].try_into().unwrap()) as usize;
    if count > MAX_OBJECTS {
        return Err(ConfigError::InvalidObject);
    }
    let mut cursor = HEADER_LEN;
    let mut objects = BTreeMap::new();
    for _ in 0..count {
        let header = bytes
            .get(cursor..cursor + OBJECT_HEADER_LEN)
            .ok_or(ConfigError::Truncated)?;
        let key_len = u16::from_le_bytes(header[0..2].try_into().unwrap()) as usize;
        let schema = u16::from_le_bytes(header[2..4].try_into().unwrap());
        let flags = u32::from_le_bytes(header[4..8].try_into().unwrap());
        let payload_len = u32::from_le_bytes(header[8..12].try_into().unwrap()) as usize;
        let reserved = u32::from_le_bytes(header[12..16].try_into().unwrap());
        if reserved != 0 || payload_len > MAX_PAYLOAD_LEN {
            return Err(ConfigError::InvalidObject);
        }
        cursor += OBJECT_HEADER_LEN;
        let key_end = cursor
            .checked_add(key_len)
            .ok_or(ConfigError::InvalidObject)?;
        let payload_end = key_end
            .checked_add(payload_len)
            .ok_or(ConfigError::InvalidObject)?;
        let key = std::str::from_utf8(bytes.get(cursor..key_end).ok_or(ConfigError::Truncated)?)
            .map_err(|_| ConfigError::InvalidObject)?
            .to_owned();
        let payload = bytes
            .get(key_end..payload_end)
            .ok_or(ConfigError::Truncated)?
            .to_vec();
        if objects
            .insert(
                key.clone(),
                RawConfigObject {
                    schema,
                    flags,
                    payload,
                },
            )
            .is_some()
        {
            return Err(ConfigError::DuplicateObject(key));
        }
        cursor = payload_end;
    }
    if cursor != bytes.len() {
        return Err(ConfigError::InvalidObject);
    }
    Ok(objects)
}
