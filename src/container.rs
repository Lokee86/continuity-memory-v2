use std::fs::{File, OpenOptions};
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

const MAGIC: [u8; 8] = *b"CVA\0\r\n\x1a\n";
const HEADER_LEN: u64 = 16;
const CHUNK_HEADER_LEN: u64 = 8;
const CURRENT_VERSION: FormatVersion = FormatVersion { major: 1, minor: 0 };

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FormatVersion {
    pub major: u16,
    pub minor: u16,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ChunkRef {
    pub offset: u64,
    pub len: u64,
}

#[derive(Debug)]
pub struct Container {
    file: File,
    path: PathBuf,
    version: FormatVersion,
    pub(crate) next_version: u64,
}

impl Container {
    pub fn create(path: impl AsRef<Path>) -> Result<Self, ContainerError> {
        let path = path.as_ref();
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .create_new(true)
            .open(path)
            .map_err(ContainerError::Io)?;
        file.write_all(&encode_header(CURRENT_VERSION))?;
        file.sync_all()?;
        Ok(Self {
            file,
            path: path.to_path_buf(),
            version: CURRENT_VERSION,
            next_version: 1,
        })
    }

    pub fn open(path: impl AsRef<Path>) -> Result<Self, ContainerError> {
        let path = path.as_ref();
        let mut file = OpenOptions::new().read(true).write(true).open(path)?;
        let version = read_header(&mut file)?;
        let mut container = Self {
            file,
            path: path.to_path_buf(),
            version,
            next_version: 1,
        };
        container.chunks()?;
        container.rebuild_version_clock()?;
        Ok(container)
    }

    pub fn append(&mut self, payload: &[u8]) -> Result<ChunkRef, ContainerError> {
        let len = u64::try_from(payload.len()).map_err(|_| ContainerError::ChunkTooLarge)?;
        let offset = self.file.seek(SeekFrom::End(0))?;
        self.file.write_all(&len.to_le_bytes())?;
        self.file.write_all(payload)?;
        Ok(ChunkRef { offset, len })
    }

    pub fn read(&mut self, chunk: ChunkRef) -> Result<Vec<u8>, ContainerError> {
        self.file.seek(SeekFrom::Start(chunk.offset))?;
        let stored_len = read_u64(&mut self.file, chunk.offset)?;
        if stored_len != chunk.len {
            return Err(ContainerError::InvalidChunkRef(chunk));
        }
        let len = usize::try_from(stored_len).map_err(|_| ContainerError::ChunkTooLarge)?;
        let mut payload = vec![0_u8; len];
        self.file
            .read_exact(&mut payload)
            .map_err(|error| truncated_or_io(error, chunk.offset))?;
        Ok(payload)
    }

    pub fn chunks(&mut self) -> Result<Vec<ChunkRef>, ContainerError> {
        let file_len = self.file.metadata()?.len();
        let mut offset = HEADER_LEN;
        let mut chunks = Vec::new();
        while offset < file_len {
            if file_len - offset < CHUNK_HEADER_LEN {
                return Err(ContainerError::TruncatedChunk(offset));
            }
            self.file.seek(SeekFrom::Start(offset))?;
            let len = read_u64(&mut self.file, offset)?;
            let end = offset
                .checked_add(CHUNK_HEADER_LEN)
                .and_then(|value| value.checked_add(len))
                .ok_or(ContainerError::ChunkTooLarge)?;
            if end > file_len {
                return Err(ContainerError::TruncatedChunk(offset));
            }
            chunks.push(ChunkRef { offset, len });
            offset = end;
        }
        Ok(chunks)
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn version(&self) -> FormatVersion {
        self.version
    }

    pub fn sync(&self) -> Result<(), ContainerError> {
        self.file.sync_all().map_err(ContainerError::Io)
    }
}

#[derive(Debug)]
pub enum ContainerError {
    Io(io::Error),
    InvalidMagic,
    TruncatedHeader,
    TruncatedChunk(u64),
    UnsupportedVersion(FormatVersion),
    InvalidHeaderLength(u32),
    InvalidChunkRef(ChunkRef),
    InvalidVersionRecord,
    VersionExhausted,
    ChunkTooLarge,
}

impl From<io::Error> for ContainerError {
    fn from(value: io::Error) -> Self {
        Self::Io(value)
    }
}

fn encode_header(version: FormatVersion) -> [u8; HEADER_LEN as usize] {
    let mut header = [0_u8; HEADER_LEN as usize];
    header[..8].copy_from_slice(&MAGIC);
    header[8..10].copy_from_slice(&version.major.to_le_bytes());
    header[10..12].copy_from_slice(&version.minor.to_le_bytes());
    header[12..16].copy_from_slice(&(HEADER_LEN as u32).to_le_bytes());
    header
}

fn read_header(file: &mut File) -> Result<FormatVersion, ContainerError> {
    let mut header = [0_u8; HEADER_LEN as usize];
    file.read_exact(&mut header).map_err(|error| {
        if error.kind() == io::ErrorKind::UnexpectedEof {
            ContainerError::TruncatedHeader
        } else {
            ContainerError::Io(error)
        }
    })?;
    if header[..8] != MAGIC {
        return Err(ContainerError::InvalidMagic);
    }
    let version = FormatVersion {
        major: u16::from_le_bytes([header[8], header[9]]),
        minor: u16::from_le_bytes([header[10], header[11]]),
    };
    if version != CURRENT_VERSION {
        return Err(ContainerError::UnsupportedVersion(version));
    }
    let length = u32::from_le_bytes([header[12], header[13], header[14], header[15]]);
    if length != HEADER_LEN as u32 {
        return Err(ContainerError::InvalidHeaderLength(length));
    }
    Ok(version)
}

fn read_u64(file: &mut File, offset: u64) -> Result<u64, ContainerError> {
    let mut bytes = [0_u8; 8];
    file.read_exact(&mut bytes)
        .map_err(|error| truncated_or_io(error, offset))?;
    Ok(u64::from_le_bytes(bytes))
}

fn truncated_or_io(error: io::Error, offset: u64) -> ContainerError {
    if error.kind() == io::ErrorKind::UnexpectedEof {
        ContainerError::TruncatedChunk(offset)
    } else {
        ContainerError::Io(error)
    }
}
