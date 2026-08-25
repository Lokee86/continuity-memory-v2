use super::{DiagnosticEndpoint, support};
use continuity_memory::{
    Cva, DreamClassifier, DreamPublicationOutcome, DreamRelationKind, DreamVerificationPolicy,
    DreamVerificationVerdict, DreamVerifier, GraphRelationKind,
};
use std::error::Error;
use std::path::Path;
use support::{AUG20_NS, AUG22_NS, AUG25_NS, context, memory, reset_cva, short};

pub fn run(endpoint: &DiagnosticEndpoint, output_dir: &Path) -> Result<(), Box<dyn Error>> {
    let path = output_dir.join("duplicate-chain.cva");
    let mut cva = reset_cva(&path)?;
    let content = "Default to concise answers unless detail is requested.";
    let old = memory(
        &mut cva,
        "duplicate-old",
        "Concise answers",
        content,
        AUG20_NS,
        "knowledge",
    )?;
    let middle = memory(
        &mut cva,
        "duplicate-middle",
        "Concise answers",
        content,
        AUG22_NS,
        "extracted",
    )?;
    let new = memory(
        &mut cva,
        "duplicate-new",
        "Concise answers",
        content,
        AUG25_NS,
        "extracted",
    )?;
    let classifier = DreamClassifier::new(endpoint.clone());
    let verifier = DreamVerifier::new(endpoint.clone());
    let policy = DreamVerificationPolicy::default();

    classify_publish(&classifier, &verifier, &mut cva, new, old, policy)?;
    cva.reconcile_dream_lifecycle(new)?;
    let (middle_classification, middle_verification) =
        classify_publish(&classifier, &verifier, &mut cva, middle, old, policy)?;
    cva.reconcile_dream_lifecycle(middle)?;

    assert_chain(&cva, new, middle, old)?;
    if !cva.memory(new)?.archived || !cva.memory(middle)?.archived || cva.memory(old)?.archived {
        return Err("duplicate representative lifecycle projection is incorrect".into());
    }
    if cva.publish_dream_pair(
        &middle_classification,
        Some(&middle_verification),
        policy,
        cva.graph_version(),
    )? != DreamPublicationOutcome::NoChange
    {
        return Err("duplicate-chain publication replay was not idempotent".into());
    }
    cva.sync()?;
    drop(cva);

    let reopened = Cva::open(&path)?;
    assert_chain(&reopened, new, middle, old)?;
    println!(
        "DUPLICATE_WRITE\tnew={}\tmiddle={}\told={}\tgraph_version={}\tpass",
        short(new),
        short(middle),
        short(old),
        reopened.graph_version()
    );
    Ok(())
}

fn classify_publish(
    classifier: &DreamClassifier<DiagnosticEndpoint>,
    verifier: &DreamVerifier<DiagnosticEndpoint>,
    cva: &mut Cva,
    left: continuity_memory::MemoryId,
    right: continuity_memory::MemoryId,
    policy: DreamVerificationPolicy,
) -> Result<
    (
        continuity_memory::DreamPairClassification,
        continuity_memory::DreamPairVerification,
    ),
    Box<dyn Error>,
> {
    let left_context = context(cva, left)?;
    let right_context = context(cva, right)?;
    let classification = classifier.classify_pair(&left_context, &right_context)?;
    if classification.relation != DreamRelationKind::DuplicateOf {
        return Err(format!("expected duplicate_of, got {:?}", classification.relation).into());
    }
    let verification = verifier.verify_pair(&classification, &left_context, &right_context)?;
    if verification.verdict != DreamVerificationVerdict::Accept {
        return Err(format!("duplicate verifier returned {:?}", verification.verdict).into());
    }
    if !matches!(
        cva.publish_dream_pair(
            &classification,
            Some(&verification),
            policy,
            cva.graph_version()
        )?,
        DreamPublicationOutcome::Published(_)
    ) {
        return Err("accepted duplicate did not publish".into());
    }
    Ok((classification, verification))
}

fn assert_chain(
    cva: &Cva,
    new: continuity_memory::MemoryId,
    middle: continuity_memory::MemoryId,
    old: continuity_memory::MemoryId,
) -> Result<(), Box<dyn Error>> {
    let edges = cva
        .graph_relations()
        .into_iter()
        .filter(|relation| relation.kind == GraphRelationKind::DuplicateOf)
        .map(|relation| (relation.source, relation.target))
        .collect::<Vec<_>>();
    if edges.len() != 2 || !edges.contains(&(new, middle)) || !edges.contains(&(middle, old)) {
        return Err("duplicate chain is not newest -> middle -> oldest".into());
    }
    Ok(())
}
