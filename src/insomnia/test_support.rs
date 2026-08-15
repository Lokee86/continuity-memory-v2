use crate::{Cva, Episode, EpisodeBoundary, EpisodeConfig, EpisodeOrigin};
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

pub(super) fn test_path(name: &str) -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("continuity-insomnia-{unique}"));
    fs::create_dir_all(&dir).unwrap();
    dir.join(name)
}

pub(super) fn episode(
    cva: &mut Cva,
    conversation: &str,
    prefix: &str,
    source_ns: i64,
    origin: EpisodeOrigin,
    boundary: EpisodeBoundary,
) -> Episode {
    let user = format!("{prefix}-u");
    let assistant = format!("{prefix}-a");
    cva.append_node(
        user.clone(),
        conversation.into(),
        None,
        "user".into(),
        source_ns,
        "remember this",
    )
    .unwrap();
    cva.append_node(
        assistant.clone(),
        conversation.into(),
        Some(user),
        "assistant".into(),
        source_ns + 1,
        "ack",
    )
    .unwrap();
    cva.materialize_path_episodes(
        conversation,
        &assistant,
        EpisodeConfig::default(),
        origin,
        Some((boundary, source_ns + 2)),
    )
    .unwrap()
    .created
    .into_iter()
    .next()
    .unwrap()
}
