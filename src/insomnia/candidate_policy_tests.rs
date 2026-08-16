use super::candidate_policy::validate_semantic_authority;

#[test]
fn direct_and_adoption_have_opposite_content_source_requirements() {
    assert!(
        validate_semantic_authority(
            "adoption",
            "decision",
            "Server state",
            "alright, that works",
            "The server returns the updated Player state.",
            false,
        )
        .is_some()
    );
    assert!(
        validate_semantic_authority(
            "direct",
            "decision",
            "Server state",
            "The server returns the updated Player state.",
            "The server returns the updated Player state.",
            true,
        )
        .is_some()
    );
}

#[test]
fn bare_retention_requires_content_source_but_inline_retention_does_not() {
    assert!(
        validate_semantic_authority(
            "retention",
            "preference",
            "Preference",
            "Remember that.",
            "The user prefers concise answers.",
            false,
        )
        .is_some()
    );
    assert!(
        validate_semantic_authority(
            "retention",
            "preference",
            "Preference",
            "Remember that I prefer concise answers.",
            "The user prefers concise answers.",
            false,
        )
        .is_none()
    );
    assert!(
        validate_semantic_authority(
            "retention",
            "location",
            "Workspace root",
            "Remember this path: C:\\!bin.",
            "The workspace root is C:\\!bin.",
            false,
        )
        .is_none()
    );
}

#[test]
fn vague_turn_cannot_authorize_detailed_memory_without_provenance() {
    assert!(
        validate_semantic_authority(
            "direct",
            "decision",
            "Call resolution",
            "alright, do that then",
            "Improve macro-mediated call resolution without sacrificing precision.",
            false,
        )
        .is_some()
    );
    assert!(validate_semantic_authority(
        "direct",
        "fact",
        "Deleted source assets",
        "you fucking deleted them",
        "Local source assets were deleted from D:/space-rocks and the clone lacked them because Git ignored them.",
        false,
    )
    .is_some());
}

#[test]
fn unsupported_questions_are_rejected_but_asserted_tag_question_is_kept() {
    assert!(
        validate_semantic_authority(
            "direct",
            "decision",
            "Spawner ownership",
            "say what? add local spawners and then transfer logic to the server?",
            "Add local spawners and transfer spawn logic to the server.",
            false,
        )
        .is_some()
    );
    assert!(
        validate_semantic_authority(
            "direct",
            "constraint",
            "Server-authoritative collision",
            "they'll have to because collision is server-authoritative, right?",
            "Collision handling has to remain server-authoritative.",
            false,
        )
        .is_none()
    );
    assert!(
        validate_semantic_authority(
            "direct",
            "fact",
            "Language",
            "The project uses Rust. Is that okay?",
            "The project uses Rust.",
            false,
        )
        .is_none()
    );
    assert!(
        validate_semantic_authority(
            "direct",
            "correction",
            "TextureRect UI",
            "I'm using a TextureRect, right?",
            "The UI uses a TextureRect rather than Sprite2D.",
            false,
        )
        .is_none()
    );
    assert!(
        validate_semantic_authority(
            "direct",
            "constraint",
            "Custom room codes deferred",
            "it's also not accepting custom room codes at the moment, that's not major is it?",
            "Custom room codes are deferred.",
            false,
        )
        .is_some()
    );
}

#[test]
fn pure_receipts_are_rejected_but_resulting_state_is_kept() {
    assert!(
        validate_semantic_authority(
            "direct",
            "fact",
            "Multiplayer prompt progress",
            "completed through 41",
            "The multiplayer lifecycle prompt sequence is completed through Prompt 41.",
            false,
        )
        .is_some()
    );
    assert!(
        validate_semantic_authority(
            "direct",
            "fact",
            "Room type ownership",
            "COMPLETED PROMPT 31: networking now references rooms.Room directly",
            "Networking now references rooms.Room directly.",
            false,
        )
        .is_none()
    );
}
