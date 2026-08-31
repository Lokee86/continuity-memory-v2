use bytes::Bytes;
use lore_single_file_spike::SingleFileStore;
use lore_storage::write_tracker::WriteContext;
use lore_storage::{
    Context, ImmutableStore, Partition, ReadOptions, WriteOptions, read, write_content,
};
use std::sync::Arc;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

const OBJECTS: usize = 32;
const OBJECT_BYTES: usize = 1024 * 1024;
const RANDOM_READS: usize = 128;

fn payload(seed: u64) -> Bytes {
    let mut state = seed | 1;
    let mut out = vec![0u8; OBJECT_BYTES];
    for chunk in out.chunks_mut(8) {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        let bytes = state.to_le_bytes();
        chunk.copy_from_slice(&bytes[..chunk.len()]);
    }
    Bytes::from(out)
}

#[tokio::main(flavor = "multi_thread", worker_threads = 4)]
async fn main() {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let path = std::env::temp_dir().join(format!(
        "lore-single-file-gate-bench-{}-{stamp}.rel",
        std::process::id()
    ));
    let store = SingleFileStore::open(&path).unwrap();
    let partition = Partition::from([0xB1; 16]);
    let mut addresses = Vec::with_capacity(OBJECTS);

    let write_start = Instant::now();
    for index in 0..OBJECTS {
        let written = write_content(
            store.clone(),
            partition,
            Context::from([(index as u8).wrapping_add(1); 16]),
            payload(index as u64 + 1),
            WriteOptions::default(),
            None,
            WriteContext::none(),
            None,
        )
        .await
        .unwrap();
        addresses.push(written.address);
    }
    ImmutableStore::flush(Arc::clone(&store), true)
        .await
        .unwrap();
    let write_elapsed = write_start.elapsed();
    let artifact_bytes = store.physical_len().unwrap();
    drop(store);

    let reopen_start = Instant::now();
    let store = SingleFileStore::open(&path).unwrap();
    let reopen_elapsed = reopen_start.elapsed();

    let mut selector = 0x9E37_79B9_7F4A_7C15u64;
    let read_start = Instant::now();
    let mut read_bytes = 0usize;
    for _ in 0..RANDOM_READS {
        selector ^= selector << 13;
        selector ^= selector >> 7;
        selector ^= selector << 17;
        let address = addresses[(selector as usize) % addresses.len()];
        let (_, bytes) = read(
            store.clone(),
            partition,
            address,
            None,
            ReadOptions::default(),
            None,
        )
        .await
        .unwrap();
        read_bytes += bytes.len();
    }
    let read_elapsed = read_start.elapsed();

    let written_bytes = OBJECTS * OBJECT_BYTES;
    let mib = 1024.0 * 1024.0;
    let write_mib_s = (written_bytes as f64 / mib) / write_elapsed.as_secs_f64();
    let read_mib_s = (read_bytes as f64 / mib) / read_elapsed.as_secs_f64();
    let avg_random_read_ms = read_elapsed.as_secs_f64() * 1000.0 / RANDOM_READS as f64;

    println!("artifact={}", path.display());
    println!("logical_write_mib={:.2}", written_bytes as f64 / mib);
    println!("artifact_mib={:.2}", artifact_bytes as f64 / mib);
    println!("sequential_write_mib_s={write_mib_s:.2}");
    println!("reopen_ms={:.3}", reopen_elapsed.as_secs_f64() * 1000.0);
    println!("random_read_mib_s={read_mib_s:.2}");
    println!("avg_random_read_ms={avg_random_read_ms:.3}");
    println!("unique_payloads={}", store.unique_payloads());
    println!("compressed_payloads={}", store.compressed_payloads());

    let _ = std::fs::remove_file(path);
}
