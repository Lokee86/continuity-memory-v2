use crate::{Cva, EpisodeBoundary, EpisodeConfig, EpisodeOrigin, MemoryDraft, MemoryError};
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

fn test_path(name: &str) -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("continuity-memories-{unique}"));
    fs::create_dir_all(&dir).unwrap();
    dir.join(name)
}

fn seeded_episode(cva: &mut Cva) -> crate::Episode {
    cva.append_node(
        "u0".into(),
        "c1".into(),
        None,
        "user".into(),
        10,
        "Remember that Reliquary owns memory.",
    )
    .unwrap();
    cva.append_node(
        "a0".into(),
        "c1".into(),
        Some("u0".into()),
        "assistant".into(),
        20,
        "Understood.",
    )
    .unwrap();
    cva.materialize_path_episodes(
        "c1",
        "a0",
        EpisodeConfig::default(),
        EpisodeOrigin::Live,
        Some((EpisodeBoundary::Inactivity, 30)),
    )
    .unwrap()
    .created
    .into_iter()
    .next()
    .unwrap()
}

fn draft(episode: &crate::Episode, mutation_id: &str, content: &str) -> MemoryDraft {
    MemoryDraft {
        category: "decision".into(),
        memory_type: "project".into(),
        authority_kind: "direct".into(),
        title: "Reliquary memory authority".into(),
        content: content.into(),
        scope: "private".into(),
        lifecycle_state: "extracted".into(),
        archived: false,
        superseded_by: None,
        parent_id: None,
        source_node_id: Some("u0".into()),
        content_source_conversation_id: None,
        content_source_node_id: None,
        grounding_source_conversation_id: None,
        grounding_source_node_id: None,
        source_episode_id: Some(episode.id),
        mutation_id: mutation_id.into(),
        created_at_ns: 30,
        updated_at_ns: 30,
    }
}

#[test]
fn memory_revisions_are_authoritative_idempotent_and_reopenable() {
    let path = test_path("revisions.cva");
    let mut cva = Cva::create(&path).unwrap();
    let episode = seeded_episode(&mut cva);
    let first_draft = draft(
        &episode,
        "insomnia:e1:candidate-1",
        "Insomnia owns working-memory generation.",
    );
    let (first, created) = cva.publish_memory(None, 0, first_draft.clone()).unwrap();
    assert!(created);
    assert_eq!(first.revision, 1);
    assert_eq!(first.memory_version, 1);

    let (replayed, created) = cva.publish_memory(None, 0, first_draft).unwrap();
    assert!(!created);
    assert_eq!(replayed.id, first.id);
    assert_eq!(replayed.revision, 1);

    let mut second_draft = draft(
        &episode,
        "dream:memory-1:revision-2",
        "Insomnia owns working-memory generation.",
    );
    second_draft.created_at_ns = first.created_at_ns;
    second_draft.updated_at_ns = 40;
    second_draft.lifecycle_state = "reinforced".into();
    let (second, created) = cva.publish_memory(Some(first.id), 1, second_draft).unwrap();
    assert!(created);
    assert_eq!(second.revision, 2);
    assert_eq!(second.memory_version, 2);
    assert_eq!(cva.memory_stats().memories, 1);
    assert_eq!(cva.memory_stats().revisions, 2);

    let stale = cva.publish_memory(Some(first.id), 1, draft(&episode, "stale-update", "stale"));
    assert!(matches!(stale, Err(MemoryError::RevisionConflict)));

    cva.sync().unwrap();
    drop(cva);
    let mut reopened = Cva::open(&path).unwrap();
    assert_eq!(reopened.memory_version(), 2);
    assert_eq!(reopened.memory(first.id).unwrap().revision, 2);
    assert_eq!(
        reopened.memory_revision(first.id, 1).unwrap().content,
        first.content
    );
}

#[test]
fn mutation_id_replay_rejects_changed_payload() {
    let path = test_path("mutation.cva");
    let mut cva = Cva::create(&path).unwrap();
    let episode = seeded_episode(&mut cva);
    let first = draft(&episode, "same-mutation", "one");
    cva.publish_memory(None, 0, first).unwrap();
    let changed = draft(&episode, "same-mutation", "two");
    assert!(matches!(
        cva.publish_memory(None, 0, changed),
        Err(MemoryError::MutationConflict)
    ));
}

#[test]
fn memory_provenance_must_resolve_to_archive_authority() {
    let path = test_path("provenance.cva");
    let mut cva = Cva::create(&path).unwrap();
    let episode = seeded_episode(&mut cva);
    let mut invalid = draft(&episode, "bad-provenance", "bad");
    invalid.source_node_id = Some("a0".into());
    // Assistant turns may appear in an episode, but Insomnia authority must be a user turn;
    // the generic Memory store only proves exact range membership. Insomnia validation owns role authority.
    assert!(cva.publish_memory(None, 0, invalid).is_ok());

    let mut missing = draft(&episode, "missing-provenance", "bad");
    missing.source_node_id = Some("missing".into());
    assert!(matches!(
        cva.publish_memory(None, 0, missing),
        Err(MemoryError::InvalidProvenance)
    ));
}
