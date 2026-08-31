mod store;

pub use store::SingleFileStore;

#[cfg(test)]
mod revision_tests;

#[cfg(test)]
mod working_tree_tests;

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::Bytes;
    use lore_storage::write_tracker::WriteContext;
    use lore_storage::{
        Context, Hash, ImmutableStore, KeyType, MutableStore, Partition, ReadOptions, WriteOptions,
        read, write_content,
    };
    use std::sync::Arc;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn test_path(name: &str) -> std::path::PathBuf {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!(
            "lore-single-file-{name}-{}-{stamp}.rel",
            std::process::id()
        ))
    }

    #[tokio::test]
    async fn immutable_chunking_dedup_and_reopen_stay_in_one_file() {
        let path = test_path("immutable");
        let store = SingleFileStore::open(&path).unwrap();
        let partition = Partition::from([0x11; 16]);
        let context_a = Context::from([0x21; 16]);
        let context_b = Context::from([0x22; 16]);
        let context_c = Context::from([0x23; 16]);
        let mut data = vec![0u8; 1_500_000];
        for (index, byte) in data.iter_mut().enumerate() {
            *byte = ((index / 8192) % 251) as u8;
        }
        let data = Bytes::from(data);

        let first = write_content(
            store.clone(),
            partition,
            context_a,
            data.clone(),
            WriteOptions::default(),
            None,
            WriteContext::none(),
            None,
        )
        .await
        .unwrap();
        ImmutableStore::flush(Arc::clone(&store), true)
            .await
            .unwrap();
        let unique_after_first = store.unique_payloads();
        assert!(
            unique_after_first > 1,
            "large content must exercise Lore fragmentation"
        );
        assert!(
            store.compressed_payloads() > 0,
            "Lore compression must be represented in stored fragment flags"
        );

        let second = write_content(
            store.clone(),
            partition,
            context_b,
            data.clone(),
            WriteOptions::default(),
            None,
            WriteContext::none(),
            None,
        )
        .await
        .unwrap();
        ImmutableStore::flush(Arc::clone(&store), true)
            .await
            .unwrap();
        assert_eq!(first.address.hash, second.address.hash);
        assert_eq!(
            store.unique_payloads(),
            unique_after_first,
            "same content must not duplicate payload bytes"
        );

        let mut similar = data.to_vec();
        similar[700_000..704_096].fill(0xEE);
        let similar = Bytes::from(similar);
        let third = write_content(
            store.clone(),
            partition,
            context_c,
            similar.clone(),
            WriteOptions::default(),
            None,
            WriteContext::none(),
            None,
        )
        .await
        .unwrap();
        assert_ne!(third.address.hash, first.address.hash);
        let new_payloads = store.unique_payloads() - unique_after_first;
        assert!(new_payloads > 0);
        assert!(
            new_payloads < unique_after_first,
            "a small same-length binary edit should reuse some Lore chunks"
        );
        let (_, similar_loaded) = read(
            store.clone(),
            partition,
            third.address,
            None,
            ReadOptions::default(),
            None,
        )
        .await
        .unwrap();
        assert_eq!(similar_loaded, similar);

        let (_, loaded) = read(
            store.clone(),
            partition,
            second.address,
            None,
            ReadOptions::default(),
            None,
        )
        .await
        .unwrap();
        assert_eq!(loaded, data);
        drop(store);

        let reopened = SingleFileStore::open(&path).unwrap();
        assert_eq!(reopened.recovered_tail_bytes(), 0);
        let (_, loaded) = read(
            reopened.clone(),
            partition,
            first.address,
            None,
            ReadOptions::default(),
            None,
        )
        .await
        .unwrap();
        assert_eq!(loaded, data);
        assert_eq!(
            std::fs::read_dir(path.parent().unwrap())
                .unwrap()
                .filter_map(Result::ok)
                .filter(|entry| entry.path() == path)
                .count(),
            1
        );
        let _ = std::fs::remove_file(path);
    }

    #[tokio::test]
    async fn mutable_store_and_cas_survive_reopen() {
        let path = test_path("mutable");
        let store = SingleFileStore::open(&path).unwrap();
        let partition = Partition::from([0x31; 16]);
        let key = Hash::from([0x41; 32]);
        let first = Hash::from([0x51; 32]);
        let second = Hash::from([0x61; 32]);

        Arc::clone(&store)
            .store(partition, key, first, KeyType::BranchLatestPointer)
            .await
            .unwrap();
        assert_eq!(
            Arc::clone(&store)
                .compare_and_swap(partition, key, first, second, KeyType::BranchLatestPointer)
                .await
                .unwrap(),
            first
        );
        assert_eq!(
            Arc::clone(&store)
                .compare_and_swap(
                    partition,
                    key,
                    first,
                    Hash::from([0x71; 32]),
                    KeyType::BranchLatestPointer
                )
                .await
                .unwrap(),
            second
        );
        MutableStore::flush(Arc::clone(&store), true).await.unwrap();
        drop(store);

        let reopened = SingleFileStore::open(&path).unwrap();
        assert_eq!(
            Arc::clone(&reopened)
                .load(partition, key, KeyType::BranchLatestPointer)
                .await
                .unwrap(),
            second
        );
        let _ = std::fs::remove_file(path);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn concurrent_readers_share_one_artifact_while_one_writer_updates_mutable_state() {
        let path = test_path("concurrency");
        let store = SingleFileStore::open(&path).unwrap();
        let partition = Partition::from([0x72; 16]);
        let context = Context::from([0x73; 16]);
        let bytes = Bytes::from(vec![0x74; 750_000]);
        let written = write_content(
            store.clone(),
            partition,
            context,
            bytes.clone(),
            WriteOptions::default(),
            None,
            WriteContext::none(),
            None,
        )
        .await
        .unwrap();

        let mut readers = tokio::task::JoinSet::new();
        for _ in 0..8 {
            let store = store.clone();
            let bytes = bytes.clone();
            let address = written.address;
            readers.spawn(async move {
                for _ in 0..20 {
                    let (_, loaded) = read(
                        store.clone(),
                        partition,
                        address,
                        None,
                        ReadOptions::default(),
                        None,
                    )
                    .await
                    .unwrap();
                    assert_eq!(loaded, bytes);
                }
            });
        }

        let writer_store = store.clone();
        let writer = tokio::spawn(async move {
            let key = Hash::from([0x75; 32]);
            for value_byte in 1..=20u8 {
                writer_store
                    .clone()
                    .store(
                        partition,
                        key,
                        Hash::from([value_byte; 32]),
                        KeyType::BranchLatestPointer,
                    )
                    .await
                    .unwrap();
            }
        });

        while let Some(result) = readers.join_next().await {
            result.unwrap();
        }
        writer.await.unwrap();
        ImmutableStore::flush(store.clone(), true).await.unwrap();
        assert!(store.physical_len().unwrap() > 0);
        let _ = std::fs::remove_file(path);
    }

    #[tokio::test]
    async fn interrupted_tail_is_removed_on_reopen() {
        let path = test_path("recovery");
        let store = SingleFileStore::open(&path).unwrap();
        let before = store.physical_len().unwrap();
        store.append_interrupted_record_for_test(17).unwrap();
        let damaged = store.physical_len().unwrap();
        assert!(damaged > before);
        drop(store);

        let reopened = SingleFileStore::open(&path).unwrap();
        assert_eq!(reopened.physical_len().unwrap(), before);
        assert_eq!(reopened.recovered_tail_bytes(), damaged - before);
        let _ = std::fs::remove_file(path);
    }
}
