use crate::SingleFileStore;
use bytes::Bytes;
use lore_base::runtime::{LORE_CONTEXT, runtime};
use lore_base::types::{BranchPoint, Context, Hash, Partition};
use lore_revision::branch;
use lore_revision::branch::merge::{MergeScope, MergeStartOptions, merge_start};
use lore_revision::commit::commit_in_memory_revision;
use lore_revision::filter::Filter;
use lore_revision::instance::InstanceId;
use lore_revision::interface::{ExecutionContext, LoreGlobalArgs};
use lore_revision::metadata::Metadata;
use lore_revision::node::{Node, NodeFlags, ROOT_NODE};
use lore_revision::relay::EventDispatcher;
use lore_revision::repository::{
    InMemoryContext, RepositoryContext, RepositoryContextCreationArgs, RepositoryFormat,
    RepositoryWriteToken,
};
use lore_revision::state::State;
use lore_storage::write_tracker::WriteContext;
use lore_storage::{ImmutableStore, ReadOptions, WriteOptions, read, write_content};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

struct SpikeMarker;
impl InMemoryContext for SpikeMarker {}
const SPIKE_MARKER: SpikeMarker = SpikeMarker;

fn test_path(name: &str) -> std::path::PathBuf {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!(
        "lore-single-file-revision-{name}-{}-{stamp}.rel",
        std::process::id()
    ))
}

fn execution() -> Arc<ExecutionContext> {
    let mut globals = LoreGlobalArgs::default();
    globals.offline = 1;
    globals.local = 1;
    Arc::new(ExecutionContext::new_client_with_user_id(
        globals,
        EventDispatcher::no_dispatch(),
        "reliquary-lore-spike".to_string(),
    ))
}

fn repository(
    store: Arc<SingleFileStore>,
    repository_id: Partition,
    working_tree: Option<std::path::PathBuf>,
) -> Arc<RepositoryContext> {
    let immutable_store: Arc<dyn lore_storage::ImmutableStore> = store.clone();
    let mutable_store: Arc<dyn lore_storage::MutableStore> = store;
    Arc::new(
        RepositoryContext::new(RepositoryContextCreationArgs {
            path: working_tree,
            immutable_store,
            mutable_store,
            id: repository_id,
            instance_id: InstanceId::generate(),
            remote: Err(lore_transport::ProtocolError::from(
                lore_base::error::NoRemote,
            )),
            filter: Arc::<Filter>::default(),
            format: RepositoryFormat::Lore,
            filesystem_provider: None,
        })
        .with_write_token(RepositoryWriteToken::in_memory(&SPIKE_MARKER)),
    )
}

fn token() -> RepositoryWriteToken {
    RepositoryWriteToken::in_memory(&SPIKE_MARKER)
}

fn metadata(branch: Context) -> Metadata {
    let mut metadata = Metadata::new();
    metadata.set_branch(branch).unwrap();
    metadata
}

async fn stored_file(
    store: Arc<SingleFileStore>,
    repository_id: Partition,
    context: Context,
    bytes: Bytes,
    name: &str,
) -> Node {
    let written = write_content(
        store,
        repository_id,
        context,
        bytes.clone(),
        WriteOptions::default(),
        None,
        WriteContext::none(),
        None,
    )
    .await
    .unwrap();
    Node {
        flags: NodeFlags::File.bits(),
        mode: 0o644,
        size: bytes.len() as u64,
        address: written.address,
        name_hash: lore_storage::hash::hash_string(name),
        ..Default::default()
    }
}

async fn add_file(state: &State, repository: Arc<RepositoryContext>, node: Node, name: &str) {
    let node_id = state
        .node_add(repository.clone(), ROOT_NODE, node, name)
        .await
        .unwrap();
    state
        .node_mark_staged(
            repository,
            node_id,
            NodeFlags::StagedAdd,
            NodeFlags::DirtyAdd,
        )
        .await
        .unwrap();
}

#[tokio::test]
async fn pathless_lore_revisions_branches_reopen_and_copy_from_one_artifact() {
    let path = test_path("history");
    let copy_path = test_path("copy");
    let store = SingleFileStore::open(&path).unwrap();
    let repository_id = Partition::from([0x81; 16]);
    let main_branch = Context::from([0x82; 16]);
    let left_branch = Context::from([0x83; 16]);
    let right_branch = Context::from([0x84; 16]);
    let working_tree = std::env::temp_dir().join(format!(
        "lore-single-file-working-tree-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&working_tree).unwrap();
    let repo = repository(store.clone(), repository_id, None);
    let scoped_store = store.clone();
    let scoped_repo = repo.clone();
    let scoped_working_tree = working_tree.clone();

    let (first, second, left, right, merged) = runtime()
        .spawn(LORE_CONTEXT.scope(execution(), async move {
            let store = scoped_store;
            let repo = scoped_repo;
            let working_tree = scoped_working_tree;
            branch::create(
                repo.clone(),
                &token(),
                main_branch,
                "main",
                "",
                "reliquary-lore-spike",
                1,
                vec![],
                false,
                false,
            )
            .await
            .unwrap();

            let state = Arc::new(State::new());
            let first_bytes = Bytes::from(vec![0x31; 400_000]);
            add_file(
                &state,
                repo.clone(),
                stored_file(
                    store.clone(),
                    repository_id,
                    Context::from([0x91; 16]),
                    first_bytes,
                    "first.bin",
                )
                .await,
                "first.bin",
            )
            .await;
            let first = commit_in_memory_revision(
                repo.clone(),
                &token(),
                state.clone(),
                metadata(main_branch),
                Hash::default(),
                main_branch,
            )
            .await
            .unwrap();

            add_file(
                &state,
                repo.clone(),
                stored_file(
                    store.clone(),
                    repository_id,
                    Context::from([0x92; 16]),
                    Bytes::from(vec![0x32; 300_000]),
                    "second.bin",
                )
                .await,
                "second.bin",
            )
            .await;
            let second = commit_in_memory_revision(
                repo.clone(),
                &token(),
                state,
                metadata(main_branch),
                first,
                main_branch,
            )
            .await
            .unwrap();

            let stack = vec![BranchPoint {
                branch: main_branch,
                revision: first,
            }];
            branch::create(
                repo.clone(),
                &token(),
                left_branch,
                "left",
                "",
                "reliquary-lore-spike",
                2,
                stack.clone(),
                false,
                false,
            )
            .await
            .unwrap();
            branch::create(
                repo.clone(),
                &token(),
                right_branch,
                "right",
                "",
                "reliquary-lore-spike",
                2,
                stack,
                false,
                false,
            )
            .await
            .unwrap();

            let left_state = State::deserialize(repo.clone(), first).await.unwrap();
            add_file(
                &left_state,
                repo.clone(),
                stored_file(
                    store.clone(),
                    repository_id,
                    Context::from([0x93; 16]),
                    Bytes::from_static(b"left branch"),
                    "left.txt",
                )
                .await,
                "left.txt",
            )
            .await;
            let left = commit_in_memory_revision(
                repo.clone(),
                &token(),
                left_state,
                metadata(left_branch),
                first,
                left_branch,
            )
            .await
            .unwrap();

            let right_state = State::deserialize(repo.clone(), first).await.unwrap();
            add_file(
                &right_state,
                repo.clone(),
                stored_file(
                    store.clone(),
                    repository_id,
                    Context::from([0x94; 16]),
                    Bytes::from_static(b"right branch"),
                    "right.txt",
                )
                .await,
                "right.txt",
            )
            .await;
            let right = commit_in_memory_revision(
                repo.clone(),
                &token(),
                right_state,
                metadata(right_branch),
                first,
                right_branch,
            )
            .await
            .unwrap();

            assert_eq!(
                branch::load_latest(repo.clone(), main_branch)
                    .await
                    .unwrap(),
                second
            );
            assert_eq!(
                branch::load_latest(repo.clone(), left_branch)
                    .await
                    .unwrap(),
                left
            );
            assert_eq!(
                branch::load_latest(repo.clone(), right_branch)
                    .await
                    .unwrap(),
                right
            );
            assert_eq!(
                State::deserialize(repo.clone(), second)
                    .await
                    .unwrap()
                    .parent_self(),
                first
            );
            assert_eq!(
                State::deserialize(repo.clone(), left)
                    .await
                    .unwrap()
                    .parent_self(),
                first
            );
            assert_eq!(
                State::deserialize(repo.clone(), right)
                    .await
                    .unwrap()
                    .parent_self(),
                first
            );

            let diff = branch::diff3_collect(
                repo.clone(),
                right_branch,
                right,
                left_branch,
                left,
                None,
                true,
                false,
            )
            .await
            .unwrap();
            assert_eq!(diff.base, first);
            assert_eq!(diff.source, right);
            assert_eq!(diff.target, left);
            assert!(diff.conflicts.is_empty());
            assert!(!diff.changes.is_empty());

            std::fs::write(working_tree.join("first.bin"), vec![0x31; 400_000]).unwrap();
            std::fs::write(working_tree.join("left.txt"), b"left branch").unwrap();
            let merge_repo = repository(store.clone(), repository_id, Some(working_tree.clone()));
            lore_revision::instance::store_current_anchor(&merge_repo, left)
                .await
                .unwrap();
            lore_revision::instance::store_current_anchor_branch(&merge_repo, left_branch)
                .await
                .unwrap();
            let merged = merge_start(
                merge_repo.clone(),
                &token(),
                right_branch,
                MergeStartOptions {
                    message: "merge right into left".to_string(),
                    no_commit: false,
                    scope: MergeScope::MainOnly,
                },
            )
            .await
            .unwrap();
            let merge_state = State::deserialize(merge_repo.clone(), merged)
                .await
                .unwrap();
            assert_eq!(merge_state.parent_self(), left);
            assert_eq!(merge_state.parent_other(), right);
            assert!(
                merge_state
                    .find_node_link(merge_repo.clone(), "left.txt")
                    .await
                    .is_ok()
            );
            assert!(
                merge_state
                    .find_node_link(merge_repo.clone(), "right.txt")
                    .await
                    .is_ok()
            );
            assert_eq!(
                std::fs::read(working_tree.join("right.txt")).unwrap(),
                b"right branch"
            );
            assert!(!working_tree.join(".urc").exists());

            let checkout = State::deserialize(repo.clone(), first).await.unwrap();
            assert!(
                checkout
                    .find_node_link(repo.clone(), "first.bin")
                    .await
                    .is_ok()
            );
            assert!(
                checkout
                    .find_node_link(repo.clone(), "second.bin")
                    .await
                    .is_err()
            );
            (first, second, left, right, merged)
        }))
        .await
        .unwrap();

    ImmutableStore::flush(store.clone(), true).await.unwrap();
    drop(repo);
    drop(store);

    let reopened_store = SingleFileStore::open(&path).unwrap();
    let reopened_repo = repository(reopened_store.clone(), repository_id, None);
    runtime()
        .spawn(LORE_CONTEXT.scope(execution(), async move {
            let state = State::deserialize(reopened_repo.clone(), second)
                .await
                .unwrap();
            assert_eq!(state.parent_self(), first);
            let link = state
                .find_node_link(reopened_repo.clone(), "first.bin")
                .await
                .unwrap();
            let node = state.node(reopened_repo.clone(), link.node).await.unwrap();
            let (_, bytes) = read(
                reopened_store.clone(),
                repository_id,
                node.address,
                None,
                ReadOptions::default(),
                None,
            )
            .await
            .unwrap();
            assert_eq!(bytes.len(), 400_000);
            assert_eq!(
                branch::load_latest(reopened_repo.clone(), left_branch)
                    .await
                    .unwrap(),
                merged
            );
            assert_eq!(
                branch::load_latest(reopened_repo.clone(), right_branch)
                    .await
                    .unwrap(),
                right
            );
            let merge_state = State::deserialize(reopened_repo, merged).await.unwrap();
            assert_eq!(merge_state.parent_self(), left);
            assert_eq!(merge_state.parent_other(), right);
        }))
        .await
        .unwrap();

    std::fs::copy(&path, &copy_path).unwrap();
    let copied_store = SingleFileStore::open(&copy_path).unwrap();
    let copied_repo = repository(copied_store, repository_id, None);
    runtime()
        .spawn(LORE_CONTEXT.scope(execution(), async move {
            let copied = State::deserialize(copied_repo, second).await.unwrap();
            assert_eq!(copied.parent_self(), first);
        }))
        .await
        .unwrap();

    let _ = std::fs::remove_file(path);
    let _ = std::fs::remove_file(copy_path);
    let _ = std::fs::remove_dir_all(working_tree);
}
