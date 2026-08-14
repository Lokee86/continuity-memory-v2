use crate::archive_codec::{ArchiveRecord, decode_record};
use crate::archive_history_codec::{decode_archive_format, decode_record_version};
use crate::archive_object_index::{ContentIndex, FragmentIndex};
use crate::archive_record_index::{BranchIndex, NodeIndex};
use crate::archive_store::hash_content;
use crate::{Archive, ArchiveError, ArchiveRecordVersion, ChunkRef, Container};
use std::collections::{HashMap, HashSet};

pub(crate) struct ArchiveOpenState {
    contents: ContentIndex,
    nodes: NodeIndex,
    branches: BranchIndex,
    fragments: FragmentIndex,
    record_versions: Vec<ArchiveRecordVersion>,
    pending_records: HashMap<ChunkRef, ArchiveRecord>,
    versioned_records: HashSet<ChunkRef>,
    next_archive_version: u64,
    format_seen: bool,
}

impl ArchiveOpenState {
    pub(crate) fn new() -> Self {
        Self {
            contents: Default::default(),
            nodes: Default::default(),
            branches: Default::default(),
            fragments: Default::default(),
            record_versions: Vec::new(),
            pending_records: HashMap::new(),
            versioned_records: HashSet::new(),
            next_archive_version: 1,
            format_seen: false,
        }
    }

    pub(crate) fn ingest(
        &mut self,
        chunk: ChunkRef,
        payload: &[u8],
        latest_global_version: u64,
    ) -> Result<(), ArchiveError> {
        if decode_archive_format(payload)? {
            if self.format_seen {
                return Err(ArchiveError::ConflictingArchiveFormat);
            }
            self.format_seen = true;
            return Ok(());
        }
        if let Some(version) = decode_record_version(payload)? {
            self.ingest_version(chunk, version, latest_global_version)?;
            return Ok(());
        }
        match decode_record(payload)? {
            ArchiveRecord::Content(id, bytes) => {
                if hash_content(&bytes) != id {
                    return Err(ArchiveError::CorruptContent);
                }
                self.contents.insert(id, chunk);
            }
            record @ (ArchiveRecord::Node(_)
            | ArchiveRecord::Branch(_)
            | ArchiveRecord::Fragment(_)) => {
                self.pending_records.insert(chunk, record);
            }
            ArchiveRecord::Other => {}
        }
        Ok(())
    }

    pub(crate) fn finish(self, container: Container) -> Result<Archive, ArchiveError> {
        if !self.format_seen {
            return Err(ArchiveError::MissingArchiveFormat);
        }
        Ok(Archive {
            container,
            contents: self.contents,
            nodes: self.nodes,
            branches: self.branches,
            fragments: self.fragments,
            record_versions: self.record_versions,
            next_archive_version: self.next_archive_version,
        })
    }

    fn ingest_version(
        &mut self,
        metadata_chunk: ChunkRef,
        version: ArchiveRecordVersion,
        latest_global_version: u64,
    ) -> Result<(), ArchiveError> {
        if version.archive_version != self.next_archive_version
            || version.global_version == 0
            || version.global_version > latest_global_version
            || self
                .record_versions
                .last()
                .is_some_and(|last| last.global_version >= version.global_version)
            || version.record.offset >= metadata_chunk.offset
        {
            return Err(ArchiveError::InvalidArchiveRecordVersion);
        }

        if let Some(record) = self.pending_records.remove(&version.record) {
            self.apply_record(record)?;
            self.versioned_records.insert(version.record);
        } else if !self.versioned_records.contains(&version.record) {
            return Err(ArchiveError::InvalidArchiveRecordVersion);
        }

        self.record_versions.push(version);
        self.next_archive_version = self
            .next_archive_version
            .checked_add(1)
            .ok_or(ArchiveError::ArchiveVersionExhausted)?;
        Ok(())
    }

    fn apply_record(&mut self, record: ArchiveRecord) -> Result<(), ArchiveError> {
        match record {
            ArchiveRecord::Node(node) => self.nodes.insert(node).map(|_| ()),
            ArchiveRecord::Branch(branch) => {
                self.branches.put(branch);
                Ok(())
            }
            ArchiveRecord::Fragment(fragment) => self.fragments.insert(fragment).map(|_| ()),
            _ => Err(ArchiveError::InvalidArchiveRecordVersion),
        }
    }
}
