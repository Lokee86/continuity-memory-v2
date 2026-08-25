use crate::{
    Cva, DreamEvidenceSide, DreamPairClassification, DreamPairVerification, DreamPublicationError,
    DreamPublicationOutcome, DreamRelationDirection, DreamRelationKind, DreamVerificationPolicy,
    DreamVerificationVerdict, GraphRelationChange, GraphRelationKind, MemoryId,
};
use std::collections::HashSet;

type RelationIdentity = (MemoryId, MemoryId, GraphRelationKind);

impl Cva {
    pub fn publish_dream_pair(
        &mut self,
        classification: &DreamPairClassification,
        verification: Option<&DreamPairVerification>,
        policy: DreamVerificationPolicy,
        expected_graph_version: u64,
    ) -> Result<DreamPublicationOutcome, DreamPublicationError> {
        validate_classification(classification)?;
        if let Some(outcome) = verification_gate(classification, verification, policy)? {
            return Ok(outcome);
        }
        if classification.relation == DreamRelationKind::DuplicateOf {
            return Ok(DreamPublicationOutcome::DeferredDuplicate);
        }
        let current_graph_version = self.graph_version();
        if expected_graph_version != current_graph_version {
            return Err(crate::GraphError::RevisionConflict {
                expected: expected_graph_version,
                actual: current_graph_version,
            }
            .into());
        }

        let desired = desired_relations(classification)?;
        let existing: HashSet<_> = self
            .graph_relations()
            .into_iter()
            .filter(|relation| {
                is_primary_dream_kind(relation.kind)
                    && same_pair(
                        classification.a,
                        classification.b,
                        relation.source,
                        relation.target,
                    )
            })
            .map(|relation| (relation.source, relation.target, relation.kind))
            .collect();

        let mut changes = Vec::new();
        for &(source, target, kind) in existing.difference(&desired) {
            changes.push(GraphRelationChange {
                source,
                target,
                kind,
                active: false,
            });
        }
        for &(source, target, kind) in desired.difference(&existing) {
            changes.push(GraphRelationChange {
                source,
                target,
                kind,
                active: true,
            });
        }
        if changes.is_empty() {
            return Ok(DreamPublicationOutcome::NoChange);
        }
        let published = self.set_memory_relations(&changes, expected_graph_version)?;
        if published.is_empty() {
            Ok(DreamPublicationOutcome::NoChange)
        } else {
            Ok(DreamPublicationOutcome::Published(published))
        }
    }
}

fn verification_gate(
    classification: &DreamPairClassification,
    verification: Option<&DreamPairVerification>,
    policy: DreamVerificationPolicy,
) -> Result<Option<DreamPublicationOutcome>, DreamPublicationError> {
    if let Some(verification) = verification {
        if verification.a != classification.a
            || verification.b != classification.b
            || verification.classification != *classification
        {
            return Err(DreamPublicationError::VerificationMismatch);
        }
        return Ok(match verification.verdict {
            DreamVerificationVerdict::Accept => None,
            verdict => Some(DreamPublicationOutcome::Withheld(verdict)),
        });
    }
    if policy.should_verify(classification.relation) {
        Err(DreamPublicationError::MissingVerification)
    } else {
        Ok(None)
    }
}

fn validate_classification(
    classification: &DreamPairClassification,
) -> Result<(), DreamPublicationError> {
    if classification.a == classification.b {
        return Err(DreamPublicationError::InvalidClassification);
    }
    let evidence_valid = if classification.relation == DreamRelationKind::None {
        classification.evidence.is_empty()
    } else {
        classification.evidence.len() == 2
            && classification
                .evidence
                .iter()
                .filter(|item| item.side == DreamEvidenceSide::A)
                .count()
                == 1
            && classification
                .evidence
                .iter()
                .filter(|item| item.side == DreamEvidenceSide::B)
                .count()
                == 1
    };
    let valid = evidence_valid
        && match classification.relation {
            DreamRelationKind::None => classification.direction == DreamRelationDirection::None,
            DreamRelationKind::Topical
            | DreamRelationKind::Recurrent
            | DreamRelationKind::DuplicateOf => {
                classification.direction == DreamRelationDirection::Undirected
            }
            DreamRelationKind::Factual
            | DreamRelationKind::Causal
            | DreamRelationKind::Supersedes => matches!(
                classification.direction,
                DreamRelationDirection::AToB | DreamRelationDirection::BToA
            ),
        };
    valid
        .then_some(())
        .ok_or(DreamPublicationError::InvalidClassification)
}

fn desired_relations(
    classification: &DreamPairClassification,
) -> Result<HashSet<RelationIdentity>, DreamPublicationError> {
    let mut desired = HashSet::new();
    let kind = match classification.relation {
        DreamRelationKind::None => return Ok(desired),
        DreamRelationKind::Topical => GraphRelationKind::Topical,
        DreamRelationKind::Factual => GraphRelationKind::Factual,
        DreamRelationKind::Causal => GraphRelationKind::Causal,
        DreamRelationKind::Recurrent => GraphRelationKind::Recurrent,
        DreamRelationKind::Supersedes => GraphRelationKind::Supersedes,
        DreamRelationKind::DuplicateOf => return Err(DreamPublicationError::InvalidClassification),
    };
    match classification.direction {
        DreamRelationDirection::Undirected => {
            desired.insert((classification.a, classification.b, kind));
            desired.insert((classification.b, classification.a, kind));
        }
        DreamRelationDirection::AToB => {
            desired.insert((classification.a, classification.b, kind));
        }
        DreamRelationDirection::BToA => {
            desired.insert((classification.b, classification.a, kind));
        }
        DreamRelationDirection::None => return Err(DreamPublicationError::InvalidClassification),
    }
    Ok(desired)
}

fn same_pair(a: MemoryId, b: MemoryId, source: MemoryId, target: MemoryId) -> bool {
    (source == a && target == b) || (source == b && target == a)
}

fn is_primary_dream_kind(kind: GraphRelationKind) -> bool {
    matches!(
        kind,
        GraphRelationKind::Topical
            | GraphRelationKind::Factual
            | GraphRelationKind::Causal
            | GraphRelationKind::Recurrent
            | GraphRelationKind::Supersedes
    )
}
