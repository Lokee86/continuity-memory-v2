use crate::{Container, ContainerError};

const VERSION_MAGIC: [u8; 8] = *b"CVAVERS1";
const VERSION_RECORD_LEN: usize = 16;

impl Container {
    pub(crate) fn observe_version_payload(&mut self, payload: &[u8]) -> Result<(), ContainerError> {
        if let Some(version) = decode_version(payload)? {
            return self.observe_version_range(version, 1);
        }
        if let Some((start, count)) =
            crate::insomnia::completion::decode_embedded_version_range(payload)
                .map_err(|_| ContainerError::InvalidVersionRecord)?
        {
            return self.observe_version_range(start, u64::from(count));
        }
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

    pub(crate) fn next_version_candidate(&self) -> u64 {
        self.next_version
    }

    pub(crate) fn commit_embedded_version_range(
        &mut self,
        start: u64,
        count: usize,
    ) -> Result<(), ContainerError> {
        let count = u64::try_from(count).map_err(|_| ContainerError::VersionExhausted)?;
        self.observe_version_range(start, count)
    }

    pub fn latest_version(&self) -> u64 {
        self.next_version.saturating_sub(1)
    }

    fn observe_version_range(&mut self, start: u64, count: u64) -> Result<(), ContainerError> {
        if count == 0 {
            return Ok(());
        }
        if start != self.next_version {
            return Err(ContainerError::InvalidVersionRecord);
        }
        self.next_version = start
            .checked_add(count)
            .ok_or(ContainerError::VersionExhausted)?;
        Ok(())
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
