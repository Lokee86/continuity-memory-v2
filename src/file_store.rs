use crate::archive_codec::encode_file;
use crate::archive_store::{hash_content, validate_text};
use crate::{Archive, ArchiveError, Container, ContentId, FileId, StoredFile};
use sha2::{Digest, Sha256};

impl Archive {
    pub(crate) fn store_file(
        &mut self,
        container: &mut Container,
        filename: String,
        mime_type: Option<String>,
        bytes: &[u8],
    ) -> Result<StoredFile, ArchiveError> {
        validate_text(&filename, "filename")?;
        if let Some(value) = mime_type.as_deref() {
            validate_text(value, "mime type")?;
        }
        let byte_length = u64::try_from(bytes.len()).map_err(|_| ArchiveError::FieldTooLarge)?;
        let content_id = hash_content(bytes);
        let id = file_id(&filename, mime_type.as_deref(), content_id, byte_length);
        let file = StoredFile {
            id,
            content_id,
            filename,
            mime_type,
            byte_length,
        };
        if let Some(existing) = self.files.get(id) {
            return if existing == &file {
                Ok(existing.clone())
            } else {
                Err(ArchiveError::ConflictingFile)
            };
        }
        self.put_content_bytes(container, content_id, bytes)?;
        let record = container.append(&encode_file(&file)?)?;
        self.publish_record(container, record)?;
        self.files.insert(file.clone())?;
        Ok(file)
    }

    pub fn file(&self, id: FileId) -> Option<&StoredFile> {
        self.files.get(id)
    }

    pub fn files(&self) -> Vec<StoredFile> {
        self.files.iter().cloned().collect()
    }

    pub(crate) fn file_bytes(
        &self,
        container: &mut Container,
        id: FileId,
    ) -> Result<Vec<u8>, ArchiveError> {
        let file = self.files.get(id).ok_or(ArchiveError::MissingFile)?;
        let bytes = self.content_bytes(container, file.content_id)?;
        if bytes.len() as u64 != file.byte_length {
            return Err(ArchiveError::CorruptFile);
        }
        Ok(bytes)
    }

    pub(crate) fn validate_files(&self) -> Result<(), ArchiveError> {
        for file in self.files.iter() {
            self.validate_file(file)?;
        }
        Ok(())
    }

    fn validate_file(&self, file: &StoredFile) -> Result<(), ArchiveError> {
        validate_text(&file.filename, "filename")?;
        if let Some(value) = file.mime_type.as_deref() {
            validate_text(value, "mime type")?;
        }
        if file.id
            != file_id(
                &file.filename,
                file.mime_type.as_deref(),
                file.content_id,
                file.byte_length,
            )
        {
            return Err(ArchiveError::InvalidFileId);
        }
        if !self.contents.contains(file.content_id) {
            return Err(ArchiveError::MissingContent);
        }
        Ok(())
    }
}

fn file_id(
    filename: &str,
    mime_type: Option<&str>,
    content_id: ContentId,
    byte_length: u64,
) -> FileId {
    let mime_type = mime_type.unwrap_or("");
    let mut hasher = Sha256::new();
    hasher.update(b"CVAFILE1-ID\0");
    hasher.update(content_id.0);
    hasher.update(byte_length.to_le_bytes());
    hasher.update((filename.len() as u64).to_le_bytes());
    hasher.update(filename.as_bytes());
    hasher.update((mime_type.len() as u64).to_le_bytes());
    hasher.update(mime_type.as_bytes());
    FileId(hasher.finalize().into())
}
