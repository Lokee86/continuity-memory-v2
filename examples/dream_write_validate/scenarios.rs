use super::{DiagnosticEndpoint, support};
use continuity_memory::{
    Cva, DreamCandidateConfig, DreamProcessor, DreamPublicationOutcome, DreamRelationKind,
    DreamVerificationPolicy, DreamVerificationVerdict, GraphRelationKind,
};
use std::error::Error;
use std::path::Path;
use support::{AUG20_NS, AUG24_NS, AUG25_NS, install_vectors, memory, reset_cva, short};

pub fn temporal_only(
    endpoint: &DiagnosticEndpoint,
    output_dir: &Path,
) -> Result<(), Box<dyn Error>> {
    let path = output_dir.join("temporal-only.cva");
    let mut cva = reset_cva(&path)?;
    let candidate = memory(
        &mut cva,
        "temporal-candidate",
        "Façade review",
        "The façade review is scheduled for 2026-08-25.",
        AUG25_NS,
        "knowledge",
    )?;
    let source = memory(
        &mut cva,
        "temporal-source",
        "Building-envelope inspection",
        "The building-envelope inspection is tomorrow.",
        AUG24_NS,
        "extracted",
    )?;
    let profile = install_vectors(&mut cva, &[source], &[&[1.0, 0.0]])?;
    let processor = DreamProcessor::new(endpoint.clone(), endpoint.clone());
    let policy = DreamVerificationPolicy::default();
    let result = processor.process_memory(
        &mut cva,
        profile,
        source,
        DreamCandidateConfig {
            limit: 1,
            semantic_limit: 0,
            prior_semantic_quota: 0,
            lexical_limit: 0,
            temporal_limit: 1,
        },
        policy,
    )?;
    if result.candidate_count != 1 || result.pairs.len() != 1 {
        return Err("temporal-only lane did not return exactly one candidate".into());
    }
    let pair = &result.pairs[0];
    if pair.classification.relation == DreamRelationKind::None {
        return Err("Ox classified the temporal-only candidate pair as none".into());
    }
    if !matches!(pair.publication, DreamPublicationOutcome::Published(_)) {
        return Err("temporal-only accepted relation was not published".into());
    }
    if result.source.lifecycle_state != "knowledge" || result.source.archived {
        return Err("temporal-only source did not complete extracted -> knowledge".into());
    }
    assert_replay(&mut cva, pair, policy, source)?;
    cva.sync()?;
    drop(cva);

    let mut reopened = Cva::open(&path)?;
    if reopened.memory(source)?.lifecycle_state != "knowledge"
        || reopened.graph_relations().is_empty()
        || reopened.memory(candidate)?.archived
    {
        return Err("temporal publication/lifecycle state did not survive reopen".into());
    }
    println!(
        "TEMPORAL_WRITE\tsource={}\tcandidate={}\trelation={:?}\tgraph_version={}\tpass",
        short(source),
        short(candidate),
        pair.classification.relation,
        reopened.graph_version()
    );
    Ok(())
}

pub fn supersession(
    endpoint: &DiagnosticEndpoint,
    output_dir: &Path,
) -> Result<(), Box<dyn Error>> {
    let path = output_dir.join("supersession.cva");
    let mut cva = reset_cva(&path)?;
    let old = memory(
        &mut cva,
        "supersession-old",
        "West wall finish",
        "Use cedar siding on the west wall.",
        AUG20_NS,
        "knowledge",
    )?;
    let new = memory(
        &mut cva,
        "supersession-new",
        "West wall finish revision",
        "Replace the previous cedar siding requirement. Use fibre-cement siding on the west wall instead.",
        AUG25_NS,
        "extracted",
    )?;
    let profile = install_vectors(&mut cva, &[old, new], &[&[1.0, 0.0], &[1.0, 0.0]])?;
    let processor = DreamProcessor::new(endpoint.clone(), endpoint.clone());
    let policy = DreamVerificationPolicy::default();
    let result = processor.process_memory(
        &mut cva,
        profile,
        new,
        DreamCandidateConfig {
            limit: 1,
            semantic_limit: 1,
            prior_semantic_quota: 0,
            lexical_limit: 0,
            temporal_limit: 0,
        },
        policy,
    )?;
    let pair = result
        .pairs
        .first()
        .ok_or("supersession pair was not evaluated")?;
    if pair.classification.relation != DreamRelationKind::Supersedes
        || pair.verification.as_ref().map(|value| value.verdict)
            != Some(DreamVerificationVerdict::Accept)
    {
        return Err(format!(
            "expected verified supersedes, got {:?}",
            pair.classification.relation
        )
        .into());
    }
    if !cva.graph_relations().iter().any(|relation| {
        relation.kind == GraphRelationKind::Supersedes
            && relation.source == new
            && relation.target == old
    }) {
        return Err("supersedes edge has the wrong semantic direction".into());
    }
    let old_memory = cva.memory(old)?;
    if !old_memory.archived || old_memory.superseded_by != Some(new) {
        return Err("superseded Memory lifecycle projection is incorrect".into());
    }
    if result.source.lifecycle_state != "knowledge" || result.source.archived {
        return Err("superseding source did not advance to active knowledge".into());
    }
    assert_replay(&mut cva, pair, policy, new)?;
    cva.sync()?;
    drop(cva);

    let mut reopened = Cva::open(&path)?;
    let old_memory = reopened.memory(old)?;
    if !old_memory.archived
        || old_memory.superseded_by != Some(new)
        || reopened.graph_relations().len() != 1
    {
        return Err("supersession state did not survive reopen".into());
    }
    println!(
        "SUPERSESSION_WRITE\tnew={}\told={}\tgraph_version={}\tpass",
        short(new),
        short(old),
        reopened.graph_version()
    );
    Ok(())
}

fn assert_replay(
    cva: &mut Cva,
    pair: &continuity_memory::DreamProcessedPair,
    policy: DreamVerificationPolicy,
    source: continuity_memory::MemoryId,
) -> Result<(), Box<dyn Error>> {
    let graph_version = cva.graph_version();
    if cva.publish_dream_pair(
        &pair.classification,
        pair.verification.as_ref(),
        policy,
        graph_version,
    )? != DreamPublicationOutcome::NoChange
        || cva.graph_version() != graph_version
    {
        return Err("publication replay was not idempotent".into());
    }
    let memory_version = cva.memory_version();
    if !cva.reconcile_dream_lifecycle(source)?.revised.is_empty()
        || cva.memory_version() != memory_version
    {
        return Err("lifecycle replay was not idempotent".into());
    }
    Ok(())
}
