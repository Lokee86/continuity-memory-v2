use crate::{ArchiveError, Cva, IncomingTurn, IngestedTurn, StoredFile};

impl Cva {
    pub fn ingest_turn(&mut self, turn: IncomingTurn) -> Result<IngestedTurn, ArchiveError> {
        self.archive
            .ingest_turn(&mut self.container, &self.project_files, turn)
    }

    pub fn files_for_source(&self, conversation_id: &str, node_id: &str) -> Vec<StoredFile> {
        self.archive.files_for_source(conversation_id, node_id)
    }
}
