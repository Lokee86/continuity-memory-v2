use crate::SingleFileStore;
use lore_base::runtime::{LORE_CONTEXT, runtime};
use lore_base::types::{Context, Partition};
use lore_revision::branch;
use lore_revision::commit::{CommitOptions, commit};
use lore_revision::file::stage::stage;
use lore_revision::filter::Filter;
use lore_revision::instance::InstanceId;
use lore_revision::interface::{ExecutionContext, LoreArray, LoreGlobalArgs, LoreString};
use lore_revision::relay::EventDispatcher;
use lore_revision::repository::{
    InMemoryContext, RepositoryContext, RepositoryContextCreationArgs, RepositoryFormat,
    RepositoryWriteToken,
};
use lore_revision::revision::sync::{SyncOptions, sync};
use lore_revision::stage::StageOptions;
use lore_revision::state::State;
use lore_storage::{ImmutableStore, ReadOptions, read};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

struct WorkingTreeMarker;
impl InMemoryContext for WorkingTreeMarker {}
const MARKER: WorkingTreeMarker = WorkingTreeMarker;

fn execution() -> Arc<ExecutionContext> {
    let mut globals = LoreGlobalArgs::default();
    globals.offline = 1;
    globals.local = 1;
    Arc::new(ExecutionContext::new_client_with_user_id(
        globals,
        EventDispatcher::no_dispatch(),
        "reliquary-working-tree-spike".into(),
    ))
}

fn repository(
    store: Arc<SingleFileStore>,
    id: Partition,
    path: std::path::PathBuf,
) -> Arc<RepositoryContext> {
    let immutable_store: Arc<dyn lore_storage::ImmutableStore> = store.clone();
    let mutable_store: Arc<dyn lore_storage::MutableStore> = store;
    Arc::new(
        RepositoryContext::new(RepositoryContextCreationArgs {
            path: Some(path),
            immutable_store,
            mutable_store,
            id,
            instance_id: InstanceId::generate(),
            remote: Err(lore_transport::ProtocolError::from(
                lore_base::error::NoRemote,
            )),
            filter: Arc::<Filter>::default(),
            format: RepositoryFormat::Lore,
            filesystem_provider: None,
        })
        .with_write_token(RepositoryWriteToken::in_memory(&MARKER)),
    )
}

fn token() -> RepositoryWriteToken {
    RepositoryWriteToken::in_memory(&MARKER)
}

fn stage_root() -> LoreArray<LoreString> {
    LoreArray::from_vec(vec![LoreString::from(".")])
}

#[tokio::test]
async fn ordinary_working_tree_round_trips_through_one_rel() {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let base = std::env::temp_dir().join(format!(
        "lore-working-tree-spike-{}-{stamp}",
        std::process::id()
    ));
    let work = base.join("project");
    let rel = base.join("project.rel");
    std::fs::create_dir_all(work.join("docs")).unwrap();
    std::fs::write(work.join("README.md"), b"alpha\n").unwrap();
    std::fs::write(work.join("docs/notes.txt"), b"one\n").unwrap();
    std::fs::write(work.join("delete-me.txt"), b"temporary\n").unwrap();
    let mut binary = vec![0u8; 512 * 1024];
    for (i, byte) in binary.iter_mut().enumerate() {
        *byte = ((i / 4096) % 251) as u8;
    }
    std::fs::write(work.join("asset.bin"), &binary).unwrap();

    let store = SingleFileStore::open(&rel).unwrap();
    let repository_id = Partition::from([0xC1; 16]);
    let main_branch = Context::from([0xC2; 16]);
    let repo = repository(store.clone(), repository_id, work.clone());
    let scoped_store = store.clone();
    let scoped_work = work.clone();

    let (first, second) = runtime()
        .spawn(LORE_CONTEXT.scope(execution(), async move {
            let store = scoped_store;
            let work = scoped_work;
            branch::create(
                repo.clone(),
                &token(),
                main_branch,
                "main",
                "",
                "reliquary-working-tree-spike",
                1,
                vec![],
                false,
                false,
            )
            .await
            .unwrap();
            lore_revision::instance::store_current_anchor(&repo, Default::default())
                .await
                .unwrap();
            lore_revision::instance::store_current_anchor_branch(&repo, main_branch)
                .await
                .unwrap();

            stage(
                repo.clone(),
                &token(),
                stage_root(),
                StageOptions {
                    scan: true,
                    ..Default::default()
                },
            )
            .await
            .unwrap();
            let first = commit(
                repo.clone(),
                &token(),
                CommitOptions::new("initial working tree".into()),
            )
            .await
            .unwrap();

            std::fs::write(work.join("README.md"), b"beta\n").unwrap();
            std::fs::rename(work.join("docs/notes.txt"), work.join("notes-renamed.txt")).unwrap();
            std::fs::create_dir_all(work.join("assets")).unwrap();
            std::fs::rename(work.join("asset.bin"), work.join("assets/asset.bin")).unwrap();
            let mut binary2 = binary.clone();
            binary2[240_000..244_096].fill(0xE7);
            std::fs::write(work.join("assets/asset.bin"), &binary2).unwrap();
            std::fs::write(work.join("new.txt"), b"new file\n").unwrap();
            std::fs::remove_file(work.join("delete-me.txt")).unwrap();

            stage(
                repo.clone(),
                &token(),
                stage_root(),
                StageOptions {
                    scan: true,
                    ..Default::default()
                },
            )
            .await
            .unwrap();
            let second = commit(
                repo.clone(),
                &token(),
                CommitOptions::new("edit rename move and binary change".into()),
            )
            .await
            .unwrap();

            let historical = State::deserialize(repo.clone(), first).await.unwrap();
            let old_readme = historical
                .find_node_link(repo.clone(), "README.md")
                .await
                .unwrap();
            let old_node = historical
                .node(repo.clone(), old_readme.node)
                .await
                .unwrap();
            let (_, old_bytes) = read(
                store.clone(),
                repository_id,
                old_node.address,
                None,
                ReadOptions::default(),
                None,
            )
            .await
            .unwrap();
            assert_eq!(old_bytes.as_ref(), b"alpha\n");
            assert!(
                historical
                    .find_node_link(repo.clone(), "docs/notes.txt")
                    .await
                    .is_ok()
            );
            assert!(
                historical
                    .find_node_link(repo.clone(), "delete-me.txt")
                    .await
                    .is_ok()
            );
            assert!(
                historical
                    .find_node_link(repo.clone(), "notes-renamed.txt")
                    .await
                    .is_err()
            );
            let latest = State::deserialize(repo.clone(), second).await.unwrap();
            assert!(
                latest
                    .find_node_link(repo.clone(), "delete-me.txt")
                    .await
                    .is_err()
            );
            (first, second)
        }))
        .await
        .unwrap();

    ImmutableStore::flush(store.clone(), true).await.unwrap();
    drop(store);
    std::fs::remove_dir_all(&work).unwrap();
    assert!(!work.exists());

    let reopened = SingleFileStore::open(&rel).unwrap();
    std::fs::create_dir_all(&work).unwrap();
    let reopened_repo = repository(reopened.clone(), repository_id, work.clone());
    runtime()
        .spawn(LORE_CONTEXT.scope(execution(), async move {
            assert_eq!(
                branch::load_latest(reopened_repo.clone(), main_branch)
                    .await
                    .unwrap(),
                second
            );
            lore_revision::instance::store_current_anchor(&reopened_repo, first)
                .await
                .unwrap();
            lore_revision::instance::store_current_anchor_branch(&reopened_repo, main_branch)
                .await
                .unwrap();
            sync(
                reopened_repo,
                &token(),
                SyncOptions {
                    revision: Some(second.to_string()),
                    reset: true,
                    force_hash_check: true,
                    ..Default::default()
                },
            )
            .await
            .unwrap();
        }))
        .await
        .unwrap();

    assert_eq!(std::fs::read(work.join("README.md")).unwrap(), b"beta\n");
    assert_eq!(
        std::fs::read(work.join("notes-renamed.txt")).unwrap(),
        b"one\n"
    );
    assert_eq!(std::fs::read(work.join("new.txt")).unwrap(), b"new file\n");
    assert!(work.join("assets/asset.bin").exists());
    assert!(!work.join("docs/notes.txt").exists());
    assert!(!work.join("delete-me.txt").exists());
    assert!(!work.join(".urc").exists());

    let _ = std::fs::remove_dir_all(base);
}
