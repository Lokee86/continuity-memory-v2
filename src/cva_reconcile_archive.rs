use crate::archive_codec::{ArchiveRecord, decode_record};
use crate::archive_history_codec::decode_record_version;
use crate::{
    ArchiveError, Branch, Cva, CvaReconcileError, Episode, FileMemoryLink, Fragment,
    IncomingAttachment, IncomingTurn, StoredFile,
};

pub(crate) enum ArchiveReplayRecord {
    Node(IncomingTurn),
    IngestedTurn(IncomingTurn),
    Branch(Branch),
    Episode(Episode),
    Fragment(Fragment),
    File(StoredFile, Vec<u8>),
}

pub(crate) struct ArchiveTail {
    pub(crate) records: Vec<ArchiveReplayRecord>,
    pub(crate) file_memory_links: Vec<FileMemoryLink>,
}

pub(crate) fn read_archive_tail(
    cva: &mut Cva,
    start_chunk: usize,
) -> Result<ArchiveTail, CvaReconcileError> {
    let chunks = cva.container.chunks()?;
    let mut records = Vec::new();
    let mut file_memory_links = Vec::new();

    for chunk in chunks.iter().skip(start_chunk) {
        let payload = cva.container.read(*chunk)?;
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
            ArchiveRecord::Episode(episode) => records.push(ArchiveReplayRecord::Episode(episode)),
            ArchiveRecord::File(file) => {
                let bytes = cva.archive.file_bytes(&mut cva.container, file.id)?;
                records.push(ArchiveReplayRecord::File(file, bytes));
            }
            ArchiveRecord::FileMemoryLink(link) => file_memory_links.push(link),
            ArchiveRecord::Fragment(fragment) => {
                records.push(ArchiveReplayRecord::Fragment(fragment))
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
        file_memory_links,
    })
}

pub(crate) fn replay_archive_tail(
    destination: &mut Cva,
    tail: &ArchiveTail,
) -> Result<usize, CvaReconcileError> {
    for record in &tail.records {
        match record {
            ArchiveReplayRecord::Node(turn) => {
                destination.append_node(
                    turn.id.clone(),
                    turn.conversation_id.clone(),
                    turn.parent_id.clone(),
                    turn.role.clone(),
                    turn.timestamp_ns,
                    &turn.content,
                )?;
            }
            ArchiveReplayRecord::IngestedTurn(turn) => {
                destination.ingest_turn(turn.clone())?;
            }
            ArchiveReplayRecord::Branch(branch) => destination.append_branch(branch.clone())?,
            ArchiveReplayRecord::Episode(episode) => {
                destination
                    .archive
                    .put_episode(&mut destination.container, episode.clone())?;
            }
            ArchiveReplayRecord::Fragment(fragment) => {
                destination
                    .archive
                    .put_fragment(&mut destination.container, fragment.clone())?;
            }
            ArchiveReplayRecord::File(file, bytes) => {
                destination.store_file(file.filename.clone(), file.mime_type.clone(), bytes)?;
            }
        }
    }
    Ok(tail.records.len())
}

pub(crate) fn replay_file_memory_links(
    destination: &mut Cva,
    links: &[FileMemoryLink],
) -> Result<usize, CvaReconcileError> {
    for link in links {
        destination.link_file_to_memory(link.file_id, link.memory_id)?;
    }
    Ok(links.len())
}
