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
use lore_single_file_spike::SingleFileStore;
use lore_storage::write_tracker::WriteContext;
use lore_storage::{ImmutableStore, WriteOptions, write_content};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

fn repository_id() -> Partition {
    Partition::from([0xD1; 16])
}
fn branch_id() -> Context {
    Context::from([0xD2; 16])
}

struct FixtureMarker;
impl InMemoryContext for FixtureMarker {}
const MARKER: FixtureMarker = FixtureMarker;

fn execution() -> Arc<ExecutionContext> {
    let mut globals = LoreGlobalArgs::default();
    globals.offline = 1;
    globals.local = 1;
    Arc::new(ExecutionContext::new_client_with_user_id(
        globals,
        EventDispatcher::no_dispatch(),
        "reliquary-cloud-transport-fixture".into(),
    ))
}

fn repository(store: Arc<SingleFileStore>) -> Arc<RepositoryContext> {
    let immutable_store: Arc<dyn lore_storage::ImmutableStore> = store.clone();
    let mutable_store: Arc<dyn lore_storage::MutableStore> = store;
    Arc::new(
        RepositoryContext::new(RepositoryContextCreationArgs {
            path: None,
            immutable_store,
            mutable_store,
            id: repository_id(),
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

fn metadata() -> Metadata {
    let mut metadata = Metadata::new();
    metadata.set_branch(branch_id()).unwrap();
    metadata
}

fn data_bytes_len(len: usize, seed: u64) -> Vec<u8> {
    let mut bytes = vec![0u8; len];
    let mut state = seed | 1;
    for chunk in bytes.chunks_mut(8) {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        let raw = state.to_le_bytes();
        chunk.copy_from_slice(&raw[..chunk.len()]);
    }
    bytes
}

fn data_bytes(mib: usize, seed: u64) -> Vec<u8> {
    data_bytes_len(mib * 1024 * 1024, seed)
}

async fn write_file(
    store: Arc<SingleFileStore>,
    context: Context,
    name: &str,
    bytes: Vec<u8>,
) -> Node {
    let written = write_content(
        store,
        repository_id(),
        context,
        Bytes::from(bytes.clone()),
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

async fn add_file(state: &State, repo: Arc<RepositoryContext>, node: Node, name: &str) {
    let id = state
        .node_add(repo.clone(), ROOT_NODE, node, name)
        .await
        .unwrap();
    state
        .node_mark_staged(repo, id, NodeFlags::StagedAdd, NodeFlags::DirtyAdd)
        .await
        .unwrap();
}

async fn init(path: PathBuf, mib: usize) {
    assert!(!path.exists(), "fixture already exists: {}", path.display());
    let store = SingleFileStore::open(&path).unwrap();
    let repo = repository(store.clone());
    branch::create(
        repo.clone(),
        &token(),
        branch_id(),
        "main",
        "",
        "reliquary-cloud-transport-fixture",
        1,
        vec![],
        false,
        false,
    )
    .await
    .unwrap();

    let state = Arc::new(State::new());
    let bytes = data_bytes(mib, 0xA55A_1234_9876_FEDC);
    let node = write_file(
        store.clone(),
        Context::from([0xD3; 16]),
        "payload.bin",
        bytes,
    )
    .await;
    add_file(&state, repo.clone(), node, "payload.bin").await;
    let revision = commit_in_memory_revision(
        repo,
        &token(),
        state,
        metadata(),
        Hash::default(),
        branch_id(),
    )
    .await
    .unwrap();
    ImmutableStore::flush(store.clone(), true).await.unwrap();
    println!("revision={revision}");
    println!("physical_bytes={}", store.physical_len().unwrap());
}

async fn append_small(path: PathBuf, tag: u8) {
    let store = SingleFileStore::open(&path).unwrap();
    let repo = repository(store.clone());
    let before = store.physical_len().unwrap();
    let current = branch::load_latest(repo.clone(), branch_id())
        .await
        .unwrap();
    let state = State::deserialize(repo.clone(), current).await.unwrap();
    let name = format!("small-{tag:03}.bin");
    let node = write_file(
        store.clone(),
        Context::from([tag; 16]),
        &name,
        data_bytes_len(64 * 1024, 0xBEEF_0000_0000_0000 | tag as u64),
    )
    .await;
    add_file(&state, repo.clone(), node, &name).await;
    let revision =
        commit_in_memory_revision(repo, &token(), state, metadata(), current, branch_id())
            .await
            .unwrap();
    ImmutableStore::flush(store.clone(), true).await.unwrap();
    let after = store.physical_len().unwrap();
    println!("revision={revision}");
    println!("before_bytes={before}");
    println!("after_bytes={after}");
    println!("delta_bytes={}", after - before);
}

async fn revise_binary(path: PathBuf, mib: usize, change_mib: usize, tag: u8) {
    assert!(change_mib <= mib);
    let store = SingleFileStore::open(&path).unwrap();
    let repo = repository(store.clone());
    let before = store.physical_len().unwrap();
    let current = branch::load_latest(repo.clone(), branch_id())
        .await
        .unwrap();
    let state = State::deserialize(repo.clone(), current).await.unwrap();
    let link = state
        .find_node_link(repo.clone(), "payload.bin")
        .await
        .unwrap();
    let old = state.node(repo.clone(), link.node).await.unwrap();

    let mut bytes = data_bytes(mib, 0xA55A_1234_9876_FEDC);
    let changed = change_mib * 1024 * 1024;
    let start = if changed == bytes.len() {
        0
    } else {
        ((tag as usize * 7 * 1024 * 1024) % (bytes.len() - changed)).min(bytes.len() - changed)
    };
    let mut replacement = data_bytes(change_mib, 0xCC00_0000_0000_0000 | tag as u64);
    bytes[start..start + changed].copy_from_slice(&replacement);
    replacement.clear();

    let written = write_content(
        store.clone(),
        repository_id(),
        old.address.context,
        Bytes::from(bytes),
        WriteOptions::default(),
        None,
        WriteContext::none(),
        None,
    )
    .await
    .unwrap();
    state
        .node_modify(repo.clone(), link.node, old.mode, old.size, written.address)
        .await
        .unwrap();
    state
        .node_mark_staged(
            repo.clone(),
            link.node,
            NodeFlags::StagedModify,
            NodeFlags::DirtyModify,
        )
        .await
        .unwrap();
    let revision =
        commit_in_memory_revision(repo, &token(), state, metadata(), current, branch_id())
            .await
            .unwrap();
    ImmutableStore::flush(store.clone(), true).await.unwrap();
    let after = store.physical_len().unwrap();
    println!("revision={revision}");
    println!("before_bytes={before}");
    println!("after_bytes={after}");
    println!("delta_bytes={}", after - before);
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    assert!(
        args.len() >= 3,
        "usage: cloud_transport_fixture <init|small|revise> <path> ..."
    );
    let command = args[1].clone();
    let path = PathBuf::from(&args[2]);
    let started = Instant::now();
    runtime().block_on(LORE_CONTEXT.scope(execution(), async move {
        match command.as_str() {
            "init" => init(path, args.get(3).and_then(|v| v.parse().ok()).unwrap_or(64)).await,
            "small" => append_small(path, args[3].parse().unwrap()).await,
            "revise" => {
                revise_binary(
                    path,
                    args[3].parse().unwrap(),
                    args[4].parse().unwrap(),
                    args[5].parse().unwrap(),
                )
                .await
            }
            _ => panic!("unknown command: {command}"),
        }
    }));
    println!(
        "operation_ms={:.3}",
        started.elapsed().as_secs_f64() * 1000.0
    );
}
