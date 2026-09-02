mod transfer;

use async_trait::async_trait;
use bytes::Bytes;
use lore_storage::{
    Address, Context, Fragment, FragmentFlags, Hash, ImmutableStore, KeyType, KeyValueStream,
    MutableStore, Partition, StoreError, StoreGetData, StoreMatch, StoreMatchResult,
    StoreObliterateStats,
};
use std::collections::BTreeMap;
use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex, RwLock};

const FILE_MAGIC: &[u8; 8] = b"RLSF0001";
const RECORD_MAGIC: &[u8; 8] = b"RLSFREC1";
const FRAME_HEADER_LEN: u64 = 8 + 1 + 8 + 32;
const PUT: u8 = 1;
const MUTABLE: u8 = 2;
const OBLITERATE: u8 = 3;
const MAX_RECORD_BODY: u64 = 2 * 1024 * 1024;

#[derive(Clone, Copy)]
struct PayloadLocation {
    fragment: Fragment,
    offset: u64,
    len: u64,
}

#[derive(Default)]
struct Index {
    associations: BTreeMap<(Partition, Address), Fragment>,
    payloads: BTreeMap<Hash, PayloadLocation>,
    mutable: BTreeMap<(Partition, u8, Hash), Hash>,
}

pub struct SingleFileStore {
    path: PathBuf,
    writer: Mutex<File>,
    index: RwLock<Index>,
    recovered_tail_bytes: u64,
}

impl SingleFileStore {
    pub fn open(path: impl AsRef<Path>) -> Result<Arc<Self>, StoreError> {
        let path = path.as_ref().to_path_buf();
        let mut file = OpenOptions::new()
            .create(true)
            .read(true)
            .write(true)
            .open(&path)
            .map_err(io_error)?;
        if file.metadata().map_err(io_error)?.len() == 0 {
            file.write_all(FILE_MAGIC).map_err(io_error)?;
            file.sync_data().map_err(io_error)?;
        }
        file.seek(SeekFrom::Start(0)).map_err(io_error)?;
        let mut magic = [0u8; 8];
        file.read_exact(&mut magic).map_err(io_error)?;
        if &magic != FILE_MAGIC {
            return Err(StoreError::internal(
                "single-file Lore store has invalid header",
            ));
        }

        let (index, valid_len, recovered_tail_bytes) = scan(&mut file)?;
        let physical_len = file.metadata().map_err(io_error)?.len();
        if valid_len < physical_len {
            file.set_len(valid_len).map_err(io_error)?;
            file.sync_data().map_err(io_error)?;
        }
        file.seek(SeekFrom::End(0)).map_err(io_error)?;
        Ok(Arc::new(Self {
            path,
            writer: Mutex::new(file),
            index: RwLock::new(index),
            recovered_tail_bytes,
        }))
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn recovered_tail_bytes(&self) -> u64 {
        self.recovered_tail_bytes
    }

    pub fn physical_len(&self) -> Result<u64, StoreError> {
        std::fs::metadata(&self.path)
            .map(|m| m.len())
            .map_err(io_error)
    }

    pub fn immutable_associations(&self) -> usize {
        self.index.read().unwrap().associations.len()
    }

    pub fn unique_payloads(&self) -> usize {
        self.index.read().unwrap().payloads.len()
    }

    pub fn compressed_payloads(&self) -> usize {
        self.index
            .read()
            .unwrap()
            .payloads
            .values()
            .filter(|payload| payload.fragment.flags & FragmentFlags::PayloadCompressed.bits() != 0)
            .count()
    }

    pub fn append_interrupted_record_for_test(&self, body_bytes: usize) -> Result<(), StoreError> {
        let mut writer = self.writer.lock().unwrap();
        writer.seek(SeekFrom::End(0)).map_err(io_error)?;
        writer.write_all(RECORD_MAGIC).map_err(io_error)?;
        writer.write_all(&[PUT]).map_err(io_error)?;
        writer
            .write_all(&(128_u64).to_le_bytes())
            .map_err(io_error)?;
        writer.write_all(&[0x55; 32]).map_err(io_error)?;
        writer
            .write_all(&vec![0xAA; body_bytes.min(64)])
            .map_err(io_error)?;
        writer.sync_data().map_err(io_error)
    }

    fn read_payload(&self, location: PayloadLocation) -> Result<Bytes, StoreError> {
        let file = File::open(&self.path).map_err(io_error)?;
        let len = usize::try_from(location.len)
            .map_err(|_| StoreError::internal("payload length does not fit usize"))?;
        let mut buf = vec![0u8; len];
        read_exact_at(&file, &mut buf, location.offset).map_err(io_error)?;
        Ok(Bytes::from(buf))
    }

    fn append(&self, kind: u8, body: &[u8]) -> Result<u64, StoreError> {
        if body.len() as u64 > MAX_RECORD_BODY {
            return Err(StoreError::internal(
                "single-file record exceeds spike limit",
            ));
        }
        let checksum = blake3::hash(body);
        let mut writer = self.writer.lock().unwrap();
        let start = writer.seek(SeekFrom::End(0)).map_err(io_error)?;
        writer.write_all(RECORD_MAGIC).map_err(io_error)?;
        writer.write_all(&[kind]).map_err(io_error)?;
        writer
            .write_all(&(body.len() as u64).to_le_bytes())
            .map_err(io_error)?;
        writer.write_all(checksum.as_bytes()).map_err(io_error)?;
        writer.write_all(body).map_err(io_error)?;
        Ok(start + FRAME_HEADER_LEN)
    }

    fn find_match(
        &self,
        partition: Partition,
        address: Address,
    ) -> Option<(StoreMatch, Partition, Context, Fragment, PayloadLocation)> {
        let index = self.index.read().unwrap();
        if let Some(fragment) = index.associations.get(&(partition, address)).copied() {
            let payload = *index.payloads.get(&address.hash)?;
            return Some((
                StoreMatch::MatchFull,
                partition,
                address.context,
                fragment,
                payload,
            ));
        }
        if let Some(((found_partition, found_address), fragment)) = index
            .associations
            .iter()
            .find(|((p, a), _)| *p == partition && a.hash == address.hash)
        {
            let payload = *index.payloads.get(&address.hash)?;
            return Some((
                StoreMatch::MatchPartition,
                *found_partition,
                found_address.context,
                *fragment,
                payload,
            ));
        }
        if let Some(((found_partition, found_address), fragment)) = index
            .associations
            .iter()
            .find(|((_p, a), _)| a.hash == address.hash)
        {
            let payload = *index.payloads.get(&address.hash)?;
            return Some((
                StoreMatch::MatchHash,
                *found_partition,
                found_address.context,
                *fragment,
                payload,
            ));
        }
        None
    }

    fn append_association(
        &self,
        partition: Partition,
        address: Address,
        mut fragment: Fragment,
        payload: Option<Bytes>,
    ) -> Result<(), StoreError> {
        let existing_payload = self
            .index
            .read()
            .unwrap()
            .payloads
            .get(&address.hash)
            .copied();
        let payload_to_write = match (existing_payload, payload) {
            (Some(existing), _) => {
                let stored_mask = FragmentFlags::PayloadStored.bits();
                fragment.flags =
                    (existing.fragment.flags & !stored_mask) | (fragment.flags & stored_mask);
                None
            }
            (None, Some(payload)) => {
                let actual = lore_storage::hash::hash_fragment(fragment, payload.as_ref())
                    .map_err(|e| StoreError::internal(format!("hash fragment: {e}")))?;
                if actual != address.hash {
                    return Err(StoreError::internal(
                        "immutable payload hash does not match address",
                    ));
                }
                Some(payload)
            }
            (None, None) => {
                return Err(StoreError::from(lore_storage::PayloadNotFound::from(
                    address.hash,
                )));
            }
        };

        let mut body = Vec::with_capacity(88 + payload_to_write.as_ref().map_or(0, Bytes::len));
        body.extend_from_slice(partition.as_ref());
        body.extend_from_slice(address.hash.as_ref());
        body.extend_from_slice(address.context.as_ref());
        body.extend_from_slice(&fragment.flags.to_le_bytes());
        body.extend_from_slice(&fragment.size_payload.to_le_bytes());
        body.extend_from_slice(&fragment.size_content.to_le_bytes());
        body.extend_from_slice(
            &(payload_to_write.as_ref().map_or(0, Bytes::len) as u64).to_le_bytes(),
        );
        let payload_body_offset = body.len() as u64;
        if let Some(payload) = payload_to_write.as_ref() {
            body.extend_from_slice(payload);
        }
        let body_start = self.append(PUT, &body)?;

        let mut index = self.index.write().unwrap();
        if let Some(payload) = payload_to_write {
            index.payloads.insert(
                address.hash,
                PayloadLocation {
                    fragment,
                    offset: body_start + payload_body_offset,
                    len: payload.len() as u64,
                },
            );
        }
        let canonical_fragment = index
            .payloads
            .get(&address.hash)
            .map(|p| {
                let stored_mask = FragmentFlags::PayloadStored.bits();
                Fragment {
                    flags: (p.fragment.flags & !stored_mask) | (fragment.flags & stored_mask),
                    ..p.fragment
                }
            })
            .unwrap_or(fragment);
        index
            .associations
            .insert((partition, address), canonical_fragment);
        Ok(())
    }

    fn append_mutable(
        &self,
        partition: Partition,
        key: Hash,
        key_type: KeyType,
        value: Hash,
    ) -> Result<(), StoreError> {
        let mut body = Vec::with_capacity(81);
        body.extend_from_slice(partition.as_ref());
        body.extend_from_slice(key.as_ref());
        body.push(key_type as u8);
        body.extend_from_slice(value.as_ref());
        self.append(MUTABLE, &body)?;
        Ok(())
    }
}

#[async_trait]
impl ImmutableStore for SingleFileStore {
    fn is_local(&self) -> bool {
        true
    }

    async fn get(
        self: Arc<Self>,
        partition: Partition,
        address: Address,
    ) -> Result<StoreGetData, StoreError> {
        let Some((match_made, found_partition, _context, fragment, payload_location)) =
            self.find_match(partition, address)
        else {
            return Err(StoreError::from(lore_storage::AddressNotFound::from(
                address,
            )));
        };
        Ok(StoreGetData {
            fragment,
            match_made,
            partition: found_partition,
            payload: Some(self.read_payload(payload_location)?),
        })
    }

    async fn query(
        self: Arc<Self>,
        partition: Partition,
        addresses: &[Address],
        results: &mut [StoreMatchResult],
    ) -> Result<(), StoreError> {
        if addresses.len() != results.len() {
            return Err(StoreError::internal("query input/output length mismatch"));
        }
        for (address, result) in addresses.iter().zip(results.iter_mut()) {
            *result = match self.find_match(partition, *address) {
                Some((match_made, found_partition, context, fragment, _)) => StoreMatchResult {
                    match_made,
                    partition: found_partition,
                    context,
                    stored_local: true,
                    stored_durable: fragment.flags & FragmentFlags::PayloadStoredDurable.bits()
                        != 0,
                },
                None => StoreMatchResult::default(),
            };
        }
        Ok(())
    }

    async fn get_metadata(
        self: Arc<Self>,
        partition: Partition,
        address: Address,
    ) -> Result<StoreGetData, StoreError> {
        let Some((match_made, found_partition, _context, fragment, _)) =
            self.find_match(partition, address)
        else {
            return Err(StoreError::from(lore_storage::AddressNotFound::from(
                address,
            )));
        };
        Ok(StoreGetData::metadata(
            fragment,
            match_made,
            found_partition,
        ))
    }

    async fn put(
        self: Arc<Self>,
        partition: Partition,
        address: Address,
        fragment: Fragment,
        payload: Option<Bytes>,
        _force: bool,
    ) -> Result<(), StoreError> {
        self.append_association(partition, address, fragment, payload)
    }

    async fn obliterate(
        self: Arc<Self>,
        partition: Partition,
        address: Address,
        stats: Arc<StoreObliterateStats>,
    ) -> Result<(), StoreError> {
        if self
            .index
            .read()
            .unwrap()
            .associations
            .contains_key(&(partition, address))
        {
            let mut body = Vec::with_capacity(64);
            body.extend_from_slice(partition.as_ref());
            body.extend_from_slice(address.as_ref());
            self.append(OBLITERATE, &body)?;
            self.index
                .write()
                .unwrap()
                .associations
                .remove(&(partition, address));
            stats.num_fragments.fetch_add(1, Ordering::Relaxed);
        }
        Ok(())
    }

    async fn evict(
        self: Arc<Self>,
        _max_capacity: usize,
        _sync_data: bool,
        _sink: Option<lore_storage::gc_event::GcEventSinkRef>,
    ) -> Result<usize, StoreError> {
        Ok(0)
    }

    async fn compact(
        self: Arc<Self>,
        _max_size: usize,
        _at: Option<usize>,
        _sync_data: bool,
        _sink: Option<lore_storage::gc_event::GcEventSinkRef>,
    ) -> Result<Option<usize>, StoreError> {
        Ok(None)
    }

    async fn compact_resume_at(self: Arc<Self>) -> Option<usize> {
        None
    }

    fn max_query_batch(&self) -> Option<usize> {
        None
    }

    async fn flush(self: Arc<Self>, sync_data: bool) -> Result<(), StoreError> {
        let writer = self.writer.lock().unwrap();
        if sync_data {
            writer.sync_all().map_err(io_error)
        } else {
            writer.sync_data().map_err(io_error)
        }
    }

    async fn fragment_count(self: Arc<Self>) -> Option<usize> {
        Some(self.immutable_associations())
    }

    async fn verify(self: Arc<Self>, _heal: bool) -> Result<(), StoreError> {
        let payloads: Vec<(Hash, PayloadLocation)> = self
            .index
            .read()
            .unwrap()
            .payloads
            .iter()
            .map(|(h, p)| (*h, *p))
            .collect();
        for (hash, location) in payloads {
            let payload = self.read_payload(location)?;
            let actual = lore_storage::hash::hash_fragment(location.fragment, payload.as_ref())
                .map_err(|e| StoreError::internal(format!("verify hash: {e}")))?;
            if actual != hash {
                return Err(StoreError::internal(
                    "single-file payload failed hash verification",
                ));
            }
        }
        Ok(())
    }

    async fn copy(
        self: Arc<Self>,
        source_partition: Partition,
        source_address: Address,
        destination_partition: Partition,
        destination_context: Context,
        durable: bool,
    ) -> Result<(), StoreError> {
        let source = {
            let index = self.index.read().unwrap();
            if source_address.context.is_zero() {
                index
                    .associations
                    .iter()
                    .find(|((p, a), _)| *p == source_partition && a.hash == source_address.hash)
                    .map(|((_k, a), f)| (*a, *f))
            } else {
                index
                    .associations
                    .get(&(source_partition, source_address))
                    .copied()
                    .map(|f| (source_address, f))
            }
        };
        let Some((source_address, mut fragment)) = source else {
            return Err(StoreError::from(lore_storage::AddressNotFound::from(
                source_address,
            )));
        };
        if durable {
            fragment.flags |= FragmentFlags::PayloadStoredDurable.bits();
        }
        self.append_association(
            destination_partition,
            Address {
                hash: source_address.hash,
                context: destination_context,
            },
            fragment,
            None,
        )
    }
}

#[async_trait]
impl MutableStore for SingleFileStore {
    async fn load(
        self: Arc<Self>,
        partition: Partition,
        key: Hash,
        key_type: KeyType,
    ) -> Result<Hash, StoreError> {
        self.index
            .read()
            .unwrap()
            .mutable
            .get(&(partition, key_type as u8, key))
            .copied()
            .ok_or_else(|| {
                StoreError::from(lore_storage::AddressNotFound::from(Address {
                    hash: key,
                    context: Context::default(),
                }))
            })
    }

    async fn store(
        self: Arc<Self>,
        partition: Partition,
        key: Hash,
        value: Hash,
        key_type: KeyType,
    ) -> Result<(), StoreError> {
        self.append_mutable(partition, key, key_type, value)?;
        let mut index = self.index.write().unwrap();
        if value.is_zero() {
            index.mutable.remove(&(partition, key_type as u8, key));
        } else {
            index
                .mutable
                .insert((partition, key_type as u8, key), value);
        }
        Ok(())
    }

    async fn compare_and_swap(
        self: Arc<Self>,
        partition: Partition,
        key: Hash,
        expected: Hash,
        value: Hash,
        key_type: KeyType,
    ) -> Result<Hash, StoreError> {
        let map_key = (partition, key_type as u8, key);
        let previous = self
            .index
            .read()
            .unwrap()
            .mutable
            .get(&map_key)
            .copied()
            .unwrap_or_default();
        if previous.is_zero() || previous == expected {
            self.append_mutable(partition, key, key_type, value)?;
            let mut index = self.index.write().unwrap();
            if value.is_zero() {
                index.mutable.remove(&map_key);
            } else {
                index.mutable.insert(map_key, value);
            }
        }
        Ok(previous)
    }

    async fn list(
        self: Arc<Self>,
        partition: Partition,
        key_type: KeyType,
    ) -> Result<KeyValueStream, StoreError> {
        let (stream, tx) = KeyValueStream::new();
        for ((p, ty, key), value) in self.index.read().unwrap().mutable.iter() {
            if (partition.is_zero() || *p == partition) && *ty == key_type as u8 {
                let _ = tx.send((*key, *value));
            }
        }
        drop(tx);
        Ok(stream)
    }

    async fn flush(self: Arc<Self>, sync_data: bool) -> Result<(), StoreError> {
        <Self as ImmutableStore>::flush(self, sync_data).await
    }
}

fn scan(file: &mut File) -> Result<(Index, u64, u64), StoreError> {
    let physical_len = file.metadata().map_err(io_error)?.len();
    let mut index = Index::default();
    let mut pos = FILE_MAGIC.len() as u64;
    while pos < physical_len {
        if physical_len - pos < FRAME_HEADER_LEN {
            return Ok((index, pos, physical_len - pos));
        }
        file.seek(SeekFrom::Start(pos)).map_err(io_error)?;
        let mut magic = [0u8; 8];
        file.read_exact(&mut magic).map_err(io_error)?;
        if &magic != RECORD_MAGIC {
            return Ok((index, pos, physical_len - pos));
        }
        let mut kind = [0u8; 1];
        file.read_exact(&mut kind).map_err(io_error)?;
        let mut len = [0u8; 8];
        file.read_exact(&mut len).map_err(io_error)?;
        let body_len = u64::from_le_bytes(len);
        let mut checksum = [0u8; 32];
        file.read_exact(&mut checksum).map_err(io_error)?;
        if body_len > MAX_RECORD_BODY || pos + FRAME_HEADER_LEN + body_len > physical_len {
            return Ok((index, pos, physical_len - pos));
        }
        let mut body = vec![0u8; body_len as usize];
        file.read_exact(&mut body).map_err(io_error)?;
        if blake3::hash(&body).as_bytes() != &checksum {
            return Ok((index, pos, physical_len - pos));
        }
        apply_record(&mut index, kind[0], &body, pos + FRAME_HEADER_LEN)?;
        pos += FRAME_HEADER_LEN + body_len;
    }
    Ok((index, pos, physical_len.saturating_sub(pos)))
}

fn apply_record(
    index: &mut Index,
    kind: u8,
    body: &[u8],
    body_start: u64,
) -> Result<(), StoreError> {
    match kind {
        PUT => {
            if body.len() < 88 {
                return Err(StoreError::internal("short immutable record"));
            }
            let partition = Partition::from(<[u8; 16]>::try_from(&body[0..16]).unwrap());
            let hash = Hash::from(<[u8; 32]>::try_from(&body[16..48]).unwrap());
            let context = Context::from(<[u8; 16]>::try_from(&body[48..64]).unwrap());
            let address = Address { hash, context };
            let fragment = Fragment {
                flags: u32::from_le_bytes(body[64..68].try_into().unwrap()),
                size_payload: u32::from_le_bytes(body[68..72].try_into().unwrap()),
                size_content: u64::from_le_bytes(body[72..80].try_into().unwrap()),
            };
            let payload_len = u64::from_le_bytes(body[80..88].try_into().unwrap());
            if payload_len > 0 {
                if body.len() != 88 + payload_len as usize {
                    return Err(StoreError::internal(
                        "immutable record payload length mismatch",
                    ));
                }
                index.payloads.insert(
                    hash,
                    PayloadLocation {
                        fragment,
                        offset: body_start + 88,
                        len: payload_len,
                    },
                );
            } else if !index.payloads.contains_key(&hash) {
                return Err(StoreError::internal(
                    "association references payload before storage",
                ));
            }
            let canonical = index.payloads.get(&hash).copied().unwrap();
            let stored_mask = FragmentFlags::PayloadStored.bits();
            let fragment = Fragment {
                flags: (canonical.fragment.flags & !stored_mask) | (fragment.flags & stored_mask),
                ..canonical.fragment
            };
            index.associations.insert((partition, address), fragment);
        }
        MUTABLE => {
            if body.len() != 81 {
                return Err(StoreError::internal("invalid mutable record length"));
            }
            let partition = Partition::from(<[u8; 16]>::try_from(&body[0..16]).unwrap());
            let key = Hash::from(<[u8; 32]>::try_from(&body[16..48]).unwrap());
            let key_type = body[48];
            let value = Hash::from(<[u8; 32]>::try_from(&body[49..81]).unwrap());
            let map_key = (partition, key_type, key);
            if value.is_zero() {
                index.mutable.remove(&map_key);
            } else {
                index.mutable.insert(map_key, value);
            }
        }
        OBLITERATE => {
            if body.len() != 64 {
                return Err(StoreError::internal("invalid obliterate record length"));
            }
            let partition = Partition::from(<[u8; 16]>::try_from(&body[0..16]).unwrap());
            let address = Address::from(&body[16..64]);
            index.associations.remove(&(partition, address));
        }
        _ => return Err(StoreError::internal("unknown single-file record kind")),
    }
    Ok(())
}

fn io_error(error: std::io::Error) -> StoreError {
    StoreError::internal(format!("single-file store I/O: {error}"))
}

#[cfg(windows)]
fn read_exact_at(file: &File, mut buf: &mut [u8], mut offset: u64) -> std::io::Result<()> {
    use std::os::windows::fs::FileExt;
    while !buf.is_empty() {
        let read = file.seek_read(buf, offset)?;
        if read == 0 {
            return Err(std::io::Error::from(std::io::ErrorKind::UnexpectedEof));
        }
        offset += read as u64;
        buf = &mut buf[read..];
    }
    Ok(())
}

#[cfg(unix)]
fn read_exact_at(file: &File, mut buf: &mut [u8], mut offset: u64) -> std::io::Result<()> {
    use std::os::unix::fs::FileExt;
    while !buf.is_empty() {
        let read = file.read_at(buf, offset)?;
        if read == 0 {
            return Err(std::io::Error::from(std::io::ErrorKind::UnexpectedEof));
        }
        offset += read as u64;
        buf = &mut buf[read..];
    }
    Ok(())
}
