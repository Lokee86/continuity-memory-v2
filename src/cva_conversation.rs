use crate::{ArchiveError, ConversationSummary, Cva, ResolvedTurn};

impl Cva {
    pub fn conversation_summaries(&self) -> Vec<ConversationSummary> {
        self.archive.conversation_summaries()
    }

    pub fn conversation_turns(
        &mut self,
        conversation_id: &str,
        leaf_node_id: &str,
    ) -> Result<Vec<ResolvedTurn>, ArchiveError> {
        self.archive
            .conversation_turns(&mut self.container, conversation_id, leaf_node_id)
    }
}
