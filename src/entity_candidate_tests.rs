use crate::entity_candidate_test_support::{
    entity_draft, install_phy_mention, install_rel_mention, publish_phy_memory, publish_rel_memory,
    temp_path,
};
use crate::{
    Cva, EntityCandidateConfig, EntityCandidateError, EntityId, GraphRelationKind,
    MemoryEntityMentionKey, Phylactery,
};

#[test]
fn exact_surface_index_is_one_to_many_revision_safe_and_reopenable() {
    let path = temp_path("surface-index.rel");
    let mut rel = Cva::create_project(&path).unwrap();
    let first = EntityId([1; 32]);
    let second = EntityId([2; 32]);
    rel.publish_entity(
        Some(first),
        0,
        entity_draft("Alpha", &["shared"], "first", 1),
    )
    .unwrap();
    rel.publish_entity(
        Some(second),
        0,
        entity_draft("Beta", &["shared"], "second", 1),
    )
    .unwrap();

    assert_eq!(
        ids(rel.entity_candidates_for_surface("SHARED", 8)),
        vec![first, second]
    );

    rel.publish_entity(
        Some(first),
        1,
        entity_draft("Alpha Prime", &["new-alias"], "first-v2", 2),
    )
    .unwrap();
    assert_eq!(
        ids(rel.entity_candidates_for_surface("shared", 8)),
        vec![second]
    );
    assert_eq!(
        ids(rel.entity_candidates_for_surface("new-alias", 8)),
        vec![first]
    );

    rel.sync().unwrap();
    drop(rel);
    let reopened = Cva::open(path).unwrap();
    assert_eq!(
        ids(reopened.entity_candidates_for_surface("new-alias", 8)),
        vec![first]
    );
}

#[test]
fn candidates_combine_exact_lexical_and_graph_evidence_without_deciding_identity() {
    let path = temp_path("candidate-evidence.rel");
    let mut rel = Cva::create_project(&path).unwrap();
    let source = publish_rel_memory(
        &mut rel,
        "source",
        "Editor selection",
        "Zephyr is the editor used by Reliquary maintainers.",
    );
    let mention = install_rel_mention(&mut rel, source, "Zephyr");
    let context_memory = publish_rel_memory(
        &mut rel,
        "context",
        "Reliquary editor",
        "Reliquary maintainers standardize the editor workspace.",
    );

    let exact = EntityId([1; 32]);
    let contextual = EntityId([2; 32]);
    rel.publish_entity(
        Some(exact),
        0,
        entity_draft("Zephyr Product", &["Zephyr"], "exact", 1),
    )
    .unwrap();
    rel.publish_entity(
        Some(contextual),
        0,
        entity_draft("Helix", &["hx"], "contextual", 1),
    )
    .unwrap();

    rel.set_memory_relation(
        source,
        context_memory,
        GraphRelationKind::Topical,
        true,
        rel.graph_version(),
    )
    .unwrap();
    rel.set_entity_association(context_memory, contextual, true, rel.graph_version())
        .unwrap();

    let first = rel
        .entity_candidates_for_mention(mention, EntityCandidateConfig::default())
        .unwrap();
    assert_eq!(first.candidates[0].entity.id, exact);
    assert!(first.candidates[0].exact_surface);

    let context = first
        .candidates
        .iter()
        .find(|candidate| candidate.entity.id == contextual)
        .unwrap();
    assert!(!context.exact_surface);
    assert_eq!(context.lexical_memory_hits, 1);
    assert_eq!(context.graph_neighbor_hits, 1);
    assert!(context.best_lexical_score > 0.0);
    assert_eq!(context.supporting_memory_ids, vec![context_memory]);

    rel.sync().unwrap();
    drop(rel);
    let mut reopened = Cva::open(path).unwrap();
    let second = reopened
        .entity_candidates_for_mention(mention, EntityCandidateConfig::default())
        .unwrap();
    assert_eq!(candidate_ids(&first), candidate_ids(&second));
    assert_eq!(first.candidates, second.candidates);
}

#[test]
fn contextual_evidence_can_promote_an_exact_collision_beyond_the_output_cap() {
    let mut rel = Cva::create_project(temp_path("surface-collision.rel")).unwrap();
    let source = publish_rel_memory(
        &mut rel,
        "source",
        "Editor selection",
        "Zephyr is the editor used by Reliquary maintainers.",
    );
    let mention = install_rel_mention(&mut rel, source, "Zephyr");
    let evidence = publish_rel_memory(
        &mut rel,
        "evidence",
        "Reliquary editor",
        "Reliquary maintainers standardize the editor workspace.",
    );

    for value in 1_u8..=9 {
        rel.publish_entity(
            Some(EntityId([value; 32])),
            0,
            entity_draft(
                &format!("Zephyr {value}"),
                &["Zephyr"],
                &format!("collision-{value}"),
                1,
            ),
        )
        .unwrap();
    }
    let contextual = EntityId([9; 32]);
    rel.set_memory_relation(
        source,
        evidence,
        GraphRelationKind::Topical,
        true,
        rel.graph_version(),
    )
    .unwrap();
    rel.set_entity_association(evidence, contextual, true, rel.graph_version())
        .unwrap();

    let result = rel
        .entity_candidates_for_mention(mention, EntityCandidateConfig::default())
        .unwrap();

    assert_eq!(
        result.candidates.len(),
        crate::MAX_ENTITY_RESOLUTION_CANDIDATES
    );
    assert_eq!(result.candidates[0].entity.id, contextual);
    assert!(result.candidates[0].exact_surface);
    assert_eq!(result.candidates[0].lexical_memory_hits, 1);
    assert_eq!(result.candidates[0].graph_neighbor_hits, 1);
}

#[test]
fn lexical_context_can_retrieve_entity_with_different_surface() {
    let mut rel = Cva::create_project(temp_path("lexical-only.rel")).unwrap();
    let source = publish_rel_memory(
        &mut rel,
        "source",
        "Editor selection",
        "Zephyr is the editor used by Reliquary maintainers.",
    );
    let mention = install_rel_mention(&mut rel, source, "Zephyr");
    let evidence = publish_rel_memory(
        &mut rel,
        "evidence",
        "Reliquary editor",
        "Reliquary maintainers standardize the editor workspace.",
    );
    let entity_id = EntityId([3; 32]);
    rel.publish_entity(
        Some(entity_id),
        0,
        entity_draft("Helix", &["hx"], "editor", 1),
    )
    .unwrap();
    rel.set_entity_association(evidence, entity_id, true, rel.graph_version())
        .unwrap();

    let mut config = EntityCandidateConfig::default();
    config.graph_neighbor_limit = 0;
    let result = rel.entity_candidates_for_mention(mention, config).unwrap();

    assert_eq!(candidate_ids(&result), vec![entity_id]);
    assert!(!result.candidates[0].exact_surface);
    assert_eq!(result.candidates[0].lexical_memory_hits, 1);
    assert_eq!(result.candidates[0].graph_neighbor_hits, 0);

    rel.set_entity_association(evidence, entity_id, false, rel.graph_version())
        .unwrap();
    let after_retraction = rel.entity_candidates_for_mention(mention, config).unwrap();
    assert!(after_retraction.candidates.is_empty());
}

#[test]
fn isolated_memory_and_phylactery_use_the_same_candidate_contract() {
    let mut phy = Phylactery::create(temp_path("candidate.phy")).unwrap();
    let memory = publish_phy_memory(&mut phy, "Zephyr is configured locally.");
    let mention = install_phy_mention(&mut phy, memory, "Zephyr");
    let entity_id = EntityId([4; 32]);
    phy.publish_entity(
        Some(entity_id),
        0,
        entity_draft("Zephyr", &[], "isolated", 1),
    )
    .unwrap();

    let result = phy
        .entity_candidates_for_mention(mention, EntityCandidateConfig::default())
        .unwrap();
    assert_eq!(candidate_ids(&result), vec![entity_id]);
    assert_eq!(result.graph_neighbors_examined, 0);
    assert!(result.candidates[0].exact_surface);
}

#[test]
fn candidate_retrieval_rejects_invalid_limits_and_unknown_mentions() {
    let mut rel = Cva::create_project(temp_path("invalid.rel")).unwrap();
    let memory = publish_rel_memory(&mut rel, "source", "Title", "Zephyr exists.");
    let key = install_rel_mention(&mut rel, memory, "Zephyr");

    let mut invalid = EntityCandidateConfig::default();
    invalid.max_candidates = crate::MAX_ENTITY_RESOLUTION_CANDIDATES + 1;
    assert!(matches!(
        rel.entity_candidates_for_mention(key, invalid),
        Err(EntityCandidateError::InvalidConfig)
    ));

    let missing = MemoryEntityMentionKey {
        start_byte: key.start_byte + 1,
        ..key
    };
    assert!(matches!(
        rel.entity_candidates_for_mention(missing, EntityCandidateConfig::default()),
        Err(EntityCandidateError::MissingMention(_))
    ));
}

fn ids(values: Vec<crate::Entity>) -> Vec<EntityId> {
    values.into_iter().map(|entity| entity.id).collect()
}

fn candidate_ids(set: &crate::EntityCandidateSet) -> Vec<EntityId> {
    set.candidates
        .iter()
        .map(|candidate| candidate.entity.id)
        .collect()
}
