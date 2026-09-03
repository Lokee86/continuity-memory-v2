use crate::archive_codec::{ArchiveRecord, decode_record};
use crate::archive_history_codec::decode_record_version;
use crate::cva_reconcile_conflict_map::archive_replay_error;
use crate::{
    ArchiveError, Branch, ConversationMetadata, Cva, CvaReconcileError, Episode, FileMemoryLink,
    Fragment, IncomingAttachment, IncomingTurn, ProjectFileRef, StoredFile,
};

pub(crate) enum ArchiveReplayRecord {
    Node(IncomingTurn),
    IngestedTurn(IncomingTurn),
    Branch(Branch),
    ConversationMetadata(ConversationMetadata),
    Episode(Episode),
    Fragment(Fragment),
    File(StoredFile, Option<Vec<u8>>, Option<ProjectFileRef>),
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
                    project_attachments: Vec::new(),
                }));
            }
            ArchiveRecord::IngestedTurn(turn) => {
                let content = cva
                    .archive
                    .content(&mut cva.container, turn.node.content_id)?;
                let mut attachments = Vec::with_capacity(turn.attachments.len());
                let mut project_attachments = Vec::new();
                for file in turn.attachments {
                    if cva.project_file_ref(file.id).is_some() {
                        project_attachments.push(file);
                    } else {
                        let bytes = cva.archive.file_bytes(&mut cva.container, file.id)?;
                        attachments.push(IncomingAttachment {
                            filename: file.filename,
                            mime_type: file.mime_type,
                            bytes,
                        });
                    }
                }
                records.push(ArchiveReplayRecord::IngestedTurn(IncomingTurn {
                    id: turn.node.id,
                    conversation_id: turn.node.conversation_id,
                    parent_id: turn.node.parent_id,
                    role: turn.node.role,
                    timestamp_ns: turn.node.timestamp_ns,
                    content,
                    attachments,
                    project_attachments,
                }));
            }
            ArchiveRecord::Branch(branch) => records.push(ArchiveReplayRecord::Branch(branch)),
            ArchiveRecord::ConversationMetadata(metadata) => {
                records.push(ArchiveReplayRecord::ConversationMetadata(metadata))
            }
            ArchiveRecord::Episode(episode) => records.push(ArchiveReplayRecord::Episode(episode)),
            ArchiveRecord::File(file) => {
                if let Some(reference) = cva.project_file_ref(file.id) {
                    records.push(ArchiveReplayRecord::File(file, None, Some(reference)));
                } else {
                    let bytes = cva.archive.file_bytes(&mut cva.container, file.id)?;
                    records.push(ArchiveReplayRecord::File(file, Some(bytes), None));
                }
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
    let before = destination.archive_version();
    for record in &tail.records {
        match record {
            ArchiveReplayRecord::Node(turn) => {
                if let Err(error) = destination.append_node(
                    turn.id.clone(),
                    turn.conversation_id.clone(),
                    turn.parent_id.clone(),
                    turn.role.clone(),
                    turn.timestamp_ns,
                    &turn.content,
                ) {
                    return Err(archive_replay_error(destination, record, error));
                }
            }
            ArchiveReplayRecord::IngestedTurn(turn) => {
                if let Err(error) = destination.ingest_turn(turn.clone()) {
                    return Err(archive_replay_error(destination, record, error));
                }
            }
            ArchiveReplayRecord::Branch(branch) => {
                if let Err(error) = destination.append_branch(branch.clone()) {
                    return Err(archive_replay_error(destination, record, error));
                }
            }
            ArchiveReplayRecord::ConversationMetadata(metadata) => {
                if let Err(error) = destination
                    .archive
                    .put_conversation_metadata(&mut destination.container, metadata.clone())
                {
                    return Err(archive_replay_error(destination, record, error));
                }
            }
            ArchiveReplayRecord::Episode(episode) => {
                if let Err(error) = destination
                    .archive
                    .put_episode(&mut destination.container, episode.clone())
                {
                    return Err(archive_replay_error(destination, record, error));
                }
            }
            ArchiveReplayRecord::Fragment(fragment) => {
                if let Err(error) = destination
                    .archive
                    .put_fragment(&mut destination.container, fragment.clone())
                {
                    return Err(archive_replay_error(destination, record, error));
                }
            }
            ArchiveReplayRecord::File(file, bytes, project_ref) => match (bytes, project_ref) {
                (Some(bytes), None) => {
                    if let Err(error) =
                        destination.store_file(file.filename.clone(), file.mime_type.clone(), bytes)
                    {
                        return Err(archive_replay_error(destination, record, error));
                    }
                }
                (None, Some(reference)) => {
                    destination.register_project_file(
                        file.filename.clone(),
                        file.mime_type.clone(),
                        file.byte_length,
                        reference.clone(),
                    )?;
                }
                _ => {
                    return Err(CvaReconcileError::UnsupportedSemanticOwner(
                        "invalid project-file replay record",
                    ));
                }
            },
        }
    }
    Ok(destination.archive_version().saturating_sub(before) as usize)
}

pub(crate) fn replay_file_memory_links(
    destination: &mut Cva,
    links: &[FileMemoryLink],
) -> Result<usize, CvaReconcileError> {
    let before = destination.archive_version();
    for link in links {
        destination.link_file_to_memory(link.file_id, link.memory_id)?;
    }
    Ok(destination.archive_version().saturating_sub(before) as usize)
}
