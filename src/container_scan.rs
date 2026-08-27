use super::{
    CHUNK_HEADER_LEN, ChunkRef, Container, ContainerError, read_header, read_u64, truncated_or_io,
};
use std::fs::OpenOptions;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

impl Container {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, ContainerError> {
        Self::open_scanned(path, |_, _, _| Ok(()))
    }

    pub(crate) fn open_scanned<E>(
        path: impl AsRef<Path>,
        mut visitor: impl FnMut(ChunkRef, &[u8], u64) -> Result<(), E>,
    ) -> Result<Self, E>
    where
        E: From<ContainerError>,
    {
        let path = path.as_ref();
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(path)
            .map_err(ContainerError::Io)?;
        let (version, identity, owner_uuid, header_len) =
            read_header(&mut file).map_err(E::from)?;
        let mut container = Self {
            file,
            path: path.to_path_buf(),
            version,
            identity,
            owner_uuid,
            header_len,
            next_version: 1,
        };
        container.scan_payloads(&mut visitor)?;
        Ok(container)
    }

    fn scan_payloads<E>(
        &mut self,
        visitor: &mut impl FnMut(ChunkRef, &[u8], u64) -> Result<(), E>,
    ) -> Result<(), E>
    where
        E: From<ContainerError>,
    {
        let file_len = self.file.metadata().map_err(ContainerError::Io)?.len();
        let mut offset = self.header_len;
        self.file
            .seek(SeekFrom::Start(offset))
            .map_err(ContainerError::Io)?;
        while offset < file_len {
            if file_len - offset < CHUNK_HEADER_LEN {
                self.recover_truncated_tail(offset).map_err(E::from)?;
                break;
            }
            let len = read_u64(&mut self.file, offset).map_err(E::from)?;
            let end = offset
                .checked_add(CHUNK_HEADER_LEN)
                .and_then(|value| value.checked_add(len))
                .ok_or_else(|| E::from(ContainerError::ChunkTooLarge))?;
            if end > file_len {
                self.recover_truncated_tail(offset).map_err(E::from)?;
                break;
            }
            let payload_len =
                usize::try_from(len).map_err(|_| E::from(ContainerError::ChunkTooLarge))?;
            let mut payload = vec![0_u8; payload_len];
            self.file
                .read_exact(&mut payload)
                .map_err(|error| E::from(truncated_or_io(error, offset)))?;
            let chunk = ChunkRef { offset, len };
            self.observe_version_payload(&payload).map_err(E::from)?;
            visitor(chunk, &payload, self.latest_version())?;
            offset = end;
        }
        Ok(())
    }

    fn recover_truncated_tail(&mut self, offset: u64) -> Result<(), ContainerError> {
        self.file.set_len(offset)?;
        self.file.seek(SeekFrom::Start(offset))?;
        self.file.sync_all()?;
        Ok(())
    }
}
