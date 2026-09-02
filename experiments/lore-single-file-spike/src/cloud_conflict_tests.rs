mod support;

use crate::SingleFileStore;
use lore_base::runtime::{LORE_CONTEXT, runtime};
use lore_base::types::{BranchPoint, Context, Partition};
use lore_revision::branch::merge::{MergeScope, MergeStartOptions, merge_start};
use lore_revision::branch::{self, BranchLatestStatus};
use lore_revision::state::State;
use lore_storage::ImmutableStore;
use support::{commit_child, create_seed, execution, path, repository, token};

#[tokio::test]
async fn two_divergent_rel_artifacts_merge_from_their_common_lore_ancestor() {
    let seed_path = path("seed");
    let left_path = path("left");
    let right_path = path("right");
    let working_tree = path("working-tree").with_extension("");
    let repository_id = Partition::from([0xA1; 16]);
    let main = Context::from([0xA2; 16]);
    let conflict = Context::from([0xA3; 16]);

    let base = create_seed(&seed_path, repository_id, main).await;
    std::fs::copy(&seed_path, &left_path).unwrap();
    std::fs::copy(&seed_path, &right_path).unwrap();

    let left_store = SingleFileStore::open(&left_path).unwrap();
    let right_store = SingleFileStore::open(&right_path).unwrap();
    let left_repo = repository(left_store.clone(), repository_id, None);
    let right_repo = repository(right_store.clone(), repository_id, None);

    let left = commit_child(
        left_store.clone(),
        left_repo,
        repository_id,
        main,
        base,
        Context::from([0xB2; 16]),
        "left.txt",
        b"left",
    )
    .await;
    let right = commit_child(
        right_store.clone(),
        right_repo,
        repository_id,
        main,
        base,
        Context::from([0xB3; 16]),
        "right.txt",
        b"right",
    )
    .await;
    assert_ne!(left, right);

    assert!(left_store.import_immutable_from(&right_store).unwrap() > 0);
    std::fs::create_dir_all(&working_tree).unwrap();
    std::fs::write(working_tree.join("base.txt"), b"base").unwrap();
    std::fs::write(working_tree.join("left.txt"), b"left").unwrap();

    let merge_repo = repository(
        left_store.clone(),
        repository_id,
        Some(working_tree.clone()),
    );
    let merged = runtime()
        .spawn(LORE_CONTEXT.scope(execution(), async move {
            branch::create(
                merge_repo.clone(),
                &token(),
                conflict,
                "device-b",
                "",
                "spike",
                2,
                vec![BranchPoint {
                    branch: main,
                    revision: base,
                }],
                false,
                false,
            )
            .await
            .unwrap();
            branch::store_latest(
                merge_repo.clone(),
                conflict,
                base,
                right,
                BranchLatestStatus::Divergent,
            )
            .await
            .unwrap();
            let diff = branch::diff3_collect(
                merge_repo.clone(),
                conflict,
                right,
                main,
                left,
                None,
                true,
                false,
            )
            .await
            .unwrap();
            assert_eq!(diff.base, base);
            assert!(diff.conflicts.is_empty());

            lore_revision::instance::store_current_anchor(&merge_repo, left)
                .await
                .unwrap();
            lore_revision::instance::store_current_anchor_branch(&merge_repo, main)
                .await
                .unwrap();
            merge_start(
                merge_repo,
                &token(),
                conflict,
                MergeStartOptions {
                    message: "merge conflicted REL".to_string(),
                    no_commit: false,
                    scope: MergeScope::MainOnly,
                },
            )
            .await
            .unwrap()
        }))
        .await
        .unwrap();

    let state = State::deserialize(repository(left_store.clone(), repository_id, None), merged)
        .await
        .unwrap();
    assert_eq!(state.parent_self(), left);
    assert_eq!(state.parent_other(), right);
    assert_eq!(
        std::fs::read(working_tree.join("right.txt")).unwrap(),
        b"right"
    );
    ImmutableStore::flush(left_store.clone(), true)
        .await
        .unwrap();
    drop(left_store);

    let reopened = SingleFileStore::open(&left_path).unwrap();
    let reopened_state = State::deserialize(repository(reopened, repository_id, None), merged)
        .await
        .unwrap();
    assert_eq!(reopened_state.parent_self(), left);
    assert_eq!(reopened_state.parent_other(), right);

    let _ = std::fs::remove_file(seed_path);
    let _ = std::fs::remove_file(left_path);
    let _ = std::fs::remove_file(right_path);
    let _ = std::fs::remove_dir_all(working_tree);
}
