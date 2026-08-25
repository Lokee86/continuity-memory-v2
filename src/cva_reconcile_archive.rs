use crate::archive_codec::{ArchiveRecord, decode_record};
use crate::archive_history_codec::decode_record_version;
use crate::insomnia::completion::decode_completion;
use crate::memory_codec::decode_version as decode_memory_version;
use crate::{
    ArchiveError, Branch, Cva, CvaReconcileError, IncomingAttachment, IncomingTurn, StoredFile,
};

pub(crate) enum ArchiveReplayRecord {
    Node(IncomingTurn),
    IngestedTurn(IncomingTurn),
    Branch(Branch),
    File(StoredFile, Vec<u8>),
}

pub(crate) struct ArchiveTail {
    pub(crate) records: Vec<ArchiveReplayRecord>,
    pub(crate) skipped_derived: usize,
}

pub(crate) fn read_archive_tail(
    cva: &mut Cva,
    start_chunk: usize,
) -> Result<ArchiveTail, CvaReconcileError> {
    let chunks = cva.container.chunks()?;
    let mut records = Vec::new();
    let mut skipped_derived = 0;

    for chunk in chunks.iter().skip(start_chunk) {
        let payload = cva.container.read(*chunk)?;
        reject_unmerged_memories(&payload)?;
        let Some(version) = decode_record_version(&payload)? else {
            continue;
        };
        let record_payload = cva.container.read(version.record)?;
        match decode_record(&record_payload)? {
            ArchiveRecord::Node(node) => {
                let content = cva.archive.content(&mut cva.container, node.content_id)?;
                records.push(ArchiveReplayRecord::Node(IncomingTurn {
                    id: node.id,
                    conversation_id: node.conversation_id,
                    parent_id: node.parent_id,
                    role: node.role,
                    timestamp_ns: node.timestamp_ns,
                    content,
                    attachments: Vec::new(),
                }));
            }
            ArchiveRecord::IngestedTurn(turn) => {
                let content = cva
                    .archive
                    .content(&mut cva.container, turn.node.content_id)?;
                let mut attachments = Vec::with_capacity(turn.attachments.len());
                for file in turn.attachments {
                    let bytes = cva.archive.file_bytes(&mut cva.container, file.id)?;
                    attachments.push(IncomingAttachment {
                        filename: file.filename,
                        mime_type: file.mime_type,
                        bytes,
                    });
                }
                records.push(ArchiveReplayRecord::IngestedTurn(IncomingTurn {
                    id: turn.node.id,
                    conversation_id: turn.node.conversation_id,
                    parent_id: turn.node.parent_id,
                    role: turn.node.role,
                    timestamp_ns: turn.node.timestamp_ns,
                    content,
                    attachments,
                }));
            }
            ArchiveRecord::Branch(branch) => records.push(ArchiveReplayRecord::Branch(branch)),
            ArchiveRecord::File(file) => {
                let bytes = cva.archive.file_bytes(&mut cva.container, file.id)?;
                records.push(ArchiveReplayRecord::File(file, bytes));
            }
            ArchiveRecord::Fragment(_) | ArchiveRecord::Episode(_) => skipped_derived += 1,
            ArchiveRecord::FileMemoryLink(_) => {
                return Err(CvaReconcileError::UnsupportedSemanticOwner(
                    "Archive file-to-Memory links",
                ));
            }
            ArchiveRecord::Content(_, _) | ArchiveRecord::Other => {
                return Err(CvaReconcileError::Archive(
                    ArchiveError::InvalidArchiveRecordVersion,
                ));
            }
        }
    }

    Ok(ArchiveTail {
        records,
        skipped_derived,
    })
}

pub(crate) fn replay_archive_tail(
    destination: &mut Cva,
    tail: ArchiveTail,
) -> Result<(usize, usize), CvaReconcileError> {
    let replayed = tail.records.len();
    for record in tail.records {
        match record {
            ArchiveReplayRecord::Node(turn) => {
                destination.append_node(
                    turn.id,
                    turn.conversation_id,
                    turn.parent_id,
                    turn.role,
                    turn.timestamp_ns,
                    &turn.content,
                )?;
            }
            ArchiveReplayRecord::IngestedTurn(turn) => {
                destination.ingest_turn(turn)?;
            }
            ArchiveReplayRecord::Branch(branch) => destination.append_branch(branch)?,
            ArchiveReplayRecord::File(file, bytes) => {
                destination.store_file(file.filename, file.mime_type, &bytes)?;
            }
        }
    }
    Ok((replayed, tail.skipped_derived))
}

fn reject_unmerged_memories(payload: &[u8]) -> Result<(), CvaReconcileError> {
    if decode_memory_version(payload)?.is_some() {
        return Err(CvaReconcileError::UnsupportedSemanticOwner("Memories"));
    }
    if decode_completion(payload)
        .map_err(CvaReconcileError::InvalidInsomniaCompletion)?
        .is_some()
    {
        return Err(CvaReconcileError::UnsupportedSemanticOwner(
            "Insomnia completions",
        ));
    }
    Ok(())
}
