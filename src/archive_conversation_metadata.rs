use crate::archive_store::validate_text;
use crate::conversation_metadata_codec::encode_conversation_metadata;
use crate::{Archive, ArchiveError, Container, ConversationMetadata};

impl Archive {
    pub(crate) fn put_conversation_metadata(
        &mut self,
        container: &mut Container,
        metadata: ConversationMetadata,
    ) -> Result<bool, ArchiveError> {
        validate_text(&metadata.conversation_id, "conversation id")?;
        if !self.has_conversation(&metadata.conversation_id) {
            return Err(ArchiveError::MissingConversation);
        }
        if let Some(title) = metadata.title.as_deref() {
            validate_text(title, "conversation title")?;
        }
        if self.conversations.get(&metadata.conversation_id) == Some(&metadata) {
            return Ok(false);
        }
        let record = container.append(&encode_conversation_metadata(&metadata)?)?;
        self.publish_record(container, record)?;
        self.conversations.put(metadata);
        Ok(true)
    }

    pub fn conversation_metadata(&self, conversation_id: &str) -> Option<&ConversationMetadata> {
        self.conversations.get(conversation_id)
    }
}
