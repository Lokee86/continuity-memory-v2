use crate::archive_codec::{ArchiveRecord, decode_record};
use crate::archive_history_codec::{encode_archive_format, encode_record_version};
use crate::{Archive, ArchiveError, ArchiveRecordVersion, Branch, ChunkRef};

impl Archive {
    pub(crate) fn initialize_history_format(&mut self) -> Result<(), ArchiveError> {
        self.container.append(&encode_archive_format())?;
        self.container.sync()?;
        Ok(())
    }

    pub fn archive_version(&self) -> u64 {
        self.next_archive_version.saturating_sub(1)
    }

    pub fn record_versions(&self) -> &[ArchiveRecordVersion] {
        &self.record_versions
    }

    pub fn record_version(&self, archive_version: u64) -> Option<&ArchiveRecordVersion> {
        let index = archive_version.checked_sub(1)? as usize;
        self.record_versions.get(index)
    }

    pub fn branch_at(
        &mut self,
        conversation_id: &str,
        branch_id: &str,
        archive_version: u64,
    ) -> Result<Option<Branch>, ArchiveError> {
        let mut found = None;
        for version in self.record_versions.clone() {
            if version.archive_version > archive_version {
                break;
            }
            if let ArchiveRecord::Branch(branch) =
                decode_record(&self.container.read(version.record)?)?
                && branch.conversation_id == conversation_id
                && branch.id == branch_id
            {
                found = Some(branch);
            }
        }
        Ok(found)
    }

    pub(crate) fn publish_record(
        &mut self,
        record: ChunkRef,
    ) -> Result<ArchiveRecordVersion, ArchiveError> {
        let archive_version = self.next_archive_version;
        let next_archive_version = archive_version
            .checked_add(1)
            .ok_or(ArchiveError::ArchiveVersionExhausted)?;
        let global_version = self.container.allocate_version()?;
        let version = ArchiveRecordVersion {
            global_version,
            archive_version,
            record,
        };
        self.container.append(&encode_record_version(version))?;
        self.record_versions.push(version);
        self.next_archive_version = next_archive_version;
        Ok(version)
    }

    pub(crate) fn insert_record_version(
        &mut self,
        version: ArchiveRecordVersion,
    ) -> Result<(), ArchiveError> {
        let expected = self.next_archive_version;
        if version.archive_version != expected {
            return Err(ArchiveError::InvalidArchiveRecordVersion);
        }
        if version.global_version == 0
            || version.global_version > self.container.latest_version()
            || self
                .record_versions
                .last()
                .is_some_and(|last| last.global_version >= version.global_version)
        {
            return Err(ArchiveError::InvalidArchiveRecordVersion);
        }
        self.next_archive_version = expected
            .checked_add(1)
            .ok_or(ArchiveError::ArchiveVersionExhausted)?;
        self.record_versions.push(version);
        Ok(())
    }

    pub(crate) fn rebuild_current_state(&mut self) -> Result<(), ArchiveError> {
        self.nodes.clear();
        self.branches.clear();
        self.fragments.clear();
        for version in self.record_versions.clone() {
            match decode_record(&self.container.read(version.record)?)? {
                ArchiveRecord::Node(node) => self.insert_rebuilt_node(node)?,
                ArchiveRecord::Branch(branch) => self.insert_rebuilt_branch_revision(branch),
                ArchiveRecord::Fragment(fragment) => self.insert_rebuilt_fragment(fragment)?,
                _ => return Err(ArchiveError::InvalidArchiveRecordVersion),
            }
        }
        Ok(())
    }
}
