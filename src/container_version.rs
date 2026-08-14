use crate::{Container, ContainerError};

const VERSION_MAGIC: [u8; 8] = *b"CVAVERS1";
const VERSION_RECORD_LEN: usize = 16;

impl Container {
    pub(crate) fn rebuild_version_clock(&mut self) -> Result<(), ContainerError> {
        let mut expected = 1_u64;
        for chunk in self.chunks()? {
            let payload = self.read(chunk)?;
            let Some(version) = decode_version(&payload)? else {
                continue;
            };
            if version != expected {
                return Err(ContainerError::InvalidVersionRecord);
            }
            expected = expected
                .checked_add(1)
                .ok_or(ContainerError::VersionExhausted)?;
        }
        self.next_version = expected;
        Ok(())
    }

    pub(crate) fn allocate_version(&mut self) -> Result<u64, ContainerError> {
        let version = self.next_version;
        let next = version
            .checked_add(1)
            .ok_or(ContainerError::VersionExhausted)?;
        self.append(&encode_version(version))?;
        self.next_version = next;
        Ok(version)
    }

    pub fn latest_version(&self) -> u64 {
        self.next_version.saturating_sub(1)
    }
}

fn encode_version(version: u64) -> [u8; VERSION_RECORD_LEN] {
    let mut out = [0_u8; VERSION_RECORD_LEN];
    out[..8].copy_from_slice(&VERSION_MAGIC);
    out[8..].copy_from_slice(&version.to_le_bytes());
    out
}

fn decode_version(payload: &[u8]) -> Result<Option<u64>, ContainerError> {
    if payload.len() < 8 || payload[..8] != VERSION_MAGIC {
        return Ok(None);
    }
    if payload.len() != VERSION_RECORD_LEN {
        return Err(ContainerError::InvalidVersionRecord);
    }
    Ok(Some(u64::from_le_bytes(payload[8..16].try_into().unwrap())))
}
