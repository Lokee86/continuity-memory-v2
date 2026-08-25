#[path = "dream_validate/support.rs"]
mod support;

use continuity_memory::{
    ContinuityConfig, DreamClassifier, DreamRelationDirection, DreamRelationKind,
    DreamVerificationVerdict, DreamVerifier, GeneralEndpoint, ModelSwitchboard,
    OpenAiReadyGeneralEndpoint,
};
use std::env;
use std::error::Error;
use std::path::PathBuf;
use support::{DiagnosticEndpoint, case, recurring_case, run_corpus_validation};

fn main() {
    if let Err(error) = run() {
        eprintln!("dream validation failed: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    let mut args = env::args().skip(1);
    let config_path = args
        .next()
        .map(PathBuf::from)
        .ok_or("usage: dream_validate <config> <cva> <fixture-json>")?;
    let cva_path = args.next().map(PathBuf::from).ok_or("missing CVA path")?;
    let fixture_path = args
        .next()
        .map(PathBuf::from)
        .ok_or("missing fixture path")?;

    let config = ContinuityConfig::open(config_path)?;
    let switchboard = ModelSwitchboard::new(config.models, config.credentials)?;
    let endpoint = DiagnosticEndpoint {
        inner: OpenAiReadyGeneralEndpoint::from_dream_switchboard(&switchboard)?,
    };
    let classifier = DreamClassifier::new(endpoint.clone());
    let verifier = DreamVerifier::new(endpoint);
    println!("model={}", classifier.model());

    run_synthetic_contract(&classifier, &verifier)?;
    run_corpus_validation(&classifier, &verifier, cva_path, fixture_path)?;
    Ok(())
}

fn run_synthetic_contract(
    classifier: &DreamClassifier<DiagnosticEndpoint>,
    verifier: &DreamVerifier<DiagnosticEndpoint>,
) -> Result<(), Box<dyn Error>> {
    let cases = vec![
        case(
            1,
            "Payroll",
            "Payroll closes Friday.",
            2,
            "West wall",
            "Use cedar siding on the west wall.",
            DreamRelationKind::None,
            DreamRelationDirection::None,
        ),
        case(
            3,
            "West wall finish",
            "Use cedar siding on the west wall.",
            4,
            "West wall flashing",
            "The west wall flashing should lap the weather membrane.",
            DreamRelationKind::Topical,
            DreamRelationDirection::Undirected,
        ),
        case(
            5,
            "Language edition",
            "The project uses Rust 2024 edition.",
            6,
            "Parser constraint",
            "The temporal parser must compile under the project's Rust 2024 edition.",
            DreamRelationKind::Factual,
            DreamRelationDirection::AToB,
        ),
        case(
            7,
            "Expired credential",
            "The OpenRouter API key expired.",
            8,
            "Validation failure",
            "Dream validation failed because the OpenRouter API key expired.",
            DreamRelationKind::Causal,
            DreamRelationDirection::AToB,
        ),
        recurring_case(),
        case(
            11,
            "Concise answers",
            "Default to concise answers unless detail is requested.",
            12,
            "Concise answers",
            "Default to concise answers unless detail is requested.",
            DreamRelationKind::DuplicateOf,
            DreamRelationDirection::Undirected,
        ),
        case(
            13,
            "Old wall finish",
            "Use cedar siding on the west wall.",
            14,
            "Wall finish revision",
            "Replace the previous cedar siding requirement; use fibre-cement siding on the west wall instead.",
            DreamRelationKind::Supersedes,
            DreamRelationDirection::BToA,
        ),
        case(
            15,
            "Parser requirement",
            "The parser must use the project's language edition.",
            16,
            "Language fact",
            "The project's language edition is Rust 2024.",
            DreamRelationKind::Factual,
            DreamRelationDirection::BToA,
        ),
        case(
            17,
            "Build failure",
            "The build failed because the generated schema was invalid.",
            18,
            "Invalid schema",
            "The generated schema was invalid.",
            DreamRelationKind::Causal,
            DreamRelationDirection::BToA,
        ),
    ];

    let mut exact = 0usize;
    let mut verifier_accepts = 0usize;
    for (index, (left, right, expected_relation, expected_direction)) in cases.iter().enumerate() {
        let result = classifier.classify_pair(left, right)?;
        let matches =
            result.relation == *expected_relation && result.direction == *expected_direction;
        exact += usize::from(matches);
        let verdict = if result.relation == DreamRelationKind::None {
            None
        } else {
            let verification = verifier.verify_pair(&result, left, right)?;
            verifier_accepts +=
                usize::from(verification.verdict == DreamVerificationVerdict::Accept);
            Some(verification.verdict)
        };
        println!(
            "SYNTHETIC\t{}\texpected={:?}/{:?}\tactual={:?}/{:?}\texact={}\tverifier={:?}",
            index + 1,
            expected_relation,
            expected_direction,
            result.relation,
            result.direction,
            matches,
            verdict
        );
    }
    println!(
        "SYNTHETIC_SUMMARY\texact={}/{}\tverifier_accepts={}/{}",
        exact,
        cases.len(),
        verifier_accepts,
        cases.len() - 1
    );
    Ok(())
}
