use crate::SingleFileStore;
use bytes::Bytes;
use lore_base::runtime::{LORE_CONTEXT, runtime};
use lore_base::types::{Context, Hash, Partition};
use lore_revision::branch;
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
use lore_storage::{WriteOptions, write_content};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

struct Marker;
impl InMemoryContext for Marker {}
const MARKER: Marker = Marker;

pub(super) fn path(name: &str) -> std::path::PathBuf {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!(
        "lore-conflict-{name}-{}-{stamp}.rel",
        std::process::id()
    ))
}

pub(super) fn execution() -> Arc<ExecutionContext> {
    let mut globals = LoreGlobalArgs::default();
    globals.offline = 1;
    globals.local = 1;
    Arc::new(ExecutionContext::new_client_with_user_id(
        globals,
        EventDispatcher::no_dispatch(),
        "reliquary-lore-conflict".to_string(),
    ))
}

pub(super) fn repository(
    store: Arc<SingleFileStore>,
    id: Partition,
    working_tree: Option<std::path::PathBuf>,
) -> Arc<RepositoryContext> {
    let immutable_store: Arc<dyn lore_storage::ImmutableStore> = store.clone();
    let mutable_store: Arc<dyn lore_storage::MutableStore> = store;
    Arc::new(
        RepositoryContext::new(RepositoryContextCreationArgs {
            path: working_tree,
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

pub(super) fn token() -> RepositoryWriteToken {
    RepositoryWriteToken::in_memory(&MARKER)
}

pub(super) async fn create_seed(
    path: &std::path::Path,
    partition: Partition,
    main: Context,
) -> Hash {
    let store = SingleFileStore::open(path).unwrap();
    let repo = repository(store.clone(), partition, None);
    runtime()
        .spawn(LORE_CONTEXT.scope(execution(), async move {
            branch::create(
                repo.clone(),
                &token(),
                main,
                "main",
                "",
                "spike",
                1,
                vec![],
                false,
                false,
            )
            .await
            .unwrap();
            let state = Arc::new(State::new());
            add_file(
                &state,
                repo.clone(),
                store,
                partition,
                Context::from([0xB1; 16]),
                "base.txt",
                Bytes::from_static(b"base"),
            )
            .await;
            commit(repo, state, main, Hash::default()).await
        }))
        .await
        .unwrap()
}

pub(super) async fn commit_child(
    store: Arc<SingleFileStore>,
    repo: Arc<RepositoryContext>,
    partition: Partition,
    branch: Context,
    parent: Hash,
    context: Context,
    name: &'static str,
    bytes: &'static [u8],
) -> Hash {
    runtime()
        .spawn(LORE_CONTEXT.scope(execution(), async move {
            let state = State::deserialize(repo.clone(), parent).await.unwrap();
            add_file(
                &state,
                repo.clone(),
                store,
                partition,
                context,
                name,
                Bytes::from_static(bytes),
            )
            .await;
            commit(repo, state, branch, parent).await
        }))
        .await
        .unwrap()
}

async fn commit(
    repo: Arc<RepositoryContext>,
    state: Arc<State>,
    branch: Context,
    parent: Hash,
) -> Hash {
    let mut metadata = Metadata::new();
    metadata.set_branch(branch).unwrap();
    commit_in_memory_revision(repo, &token(), state, metadata, parent, branch)
        .await
        .unwrap()
}

async fn add_file(
    state: &State,
    repo: Arc<RepositoryContext>,
    store: Arc<SingleFileStore>,
    partition: Partition,
    context: Context,
    name: &str,
    bytes: Bytes,
) {
    let written = write_content(
        store,
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
    let node = Node {
        flags: NodeFlags::File.bits(),
        mode: 0o644,
        size: bytes.len() as u64,
        address: written.address,
        name_hash: lore_storage::hash::hash_string(name),
        ..Default::default()
    };
    let id = state
        .node_add(repo.clone(), ROOT_NODE, node, name)
        .await
        .unwrap();
    state
        .node_mark_staged(repo, id, NodeFlags::StagedAdd, NodeFlags::DirtyAdd)
        .await
        .unwrap();
}
