use crate::{ArchiveError, FileId, FileMemoryLink, MemoryId};

pub(crate) const FILE_MEMORY_LINK_MAGIC: [u8; 8] = *b"CVAFMEM1";

pub(crate) fn encode_file_memory_link(link: FileMemoryLink) -> Vec<u8> {
    let mut out = Vec::with_capacity(72);
    out.extend_from_slice(&FILE_MEMORY_LINK_MAGIC);
    out.extend_from_slice(&link.file_id.0);
    out.extend_from_slice(&link.memory_id.0);
    out
}

pub(crate) fn decode_file_memory_link(bytes: &[u8]) -> Result<FileMemoryLink, ArchiveError> {
    if bytes.len() != 72 {
        return Err(ArchiveError::CorruptRecord("invalid file-memory link"));
    }
    Ok(FileMemoryLink {
        file_id: FileId(bytes[8..40].try_into().unwrap()),
        memory_id: MemoryId(bytes[40..72].try_into().unwrap()),
    })
}
