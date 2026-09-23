use reliquary_memory::{
    ConfiguredGeneralEndpoint, CredentialId, Cva, DreamClassifier, DreamMemoryContext,
    DreamRelationKind, GeneralEndpoint, MemoryId, ModelReasoningEffort, ModelSwitchboard,
    Phylactery, ReliquaryConfig,
};
use serde_json::{Value, json};
use std::{error::Error, fs, path::PathBuf};

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.len() < 5 {
        return Err(
            "usage: dream_omnibus_replay <fixture.json> <output.json> <config> <rel> <phy>".into(),
        );
    }
    let mut config = ReliquaryConfig::open(&args[2])?;
    let route = config
        .models
        .dream
        .as_mut()
        .ok_or("config requires dream route")?;
    route.model = "gpt-6-luna".into();
    route.credential_id = CredentialId::new("codex")?;
    route.reasoning_effort = Some(ModelReasoningEffort::Low);
    let switchboard = ModelSwitchboard::new(config.models, config.credentials)?;
    let endpoint = ConfiguredGeneralEndpoint::from_dream_switchboard(&switchboard)?;
    let model = endpoint.model().to_owned();
    let classifier = DreamClassifier::new(endpoint);
    let mut rel = Cva::open(&args[3])?;
    let mut phy = Phylactery::open(&args[4])?;

    let fixture: Value = serde_json::from_slice(&fs::read(&args[0])?)?;
    let cases = fixture["cases"]
        .as_array()
        .ok_or("fixture cases must be array")?;
    let mut rows = Vec::with_capacity(cases.len());
    let mut correct = 0usize;
    for case in cases {
        let left_id = memory_id(case["left"]["id"].as_str().ok_or("left id")?)?;
        let right_id = memory_id(case["right"]["id"].as_str().ok_or("right id")?)?;
        let left = owner_context(left_id, &mut rel, &mut phy)?;
        let right = owner_context(right_id, &mut rel, &mut phy)?;
        let result = classifier.classify_pair(&left, &right)?;
        let actual = if result.relation == DreamRelationKind::None {
            "unrelated"
        } else {
            "related"
        };
        let expected = case["expected"].as_str().ok_or("missing expected")?;
        let matched = actual == expected;
        correct += usize::from(matched);
        rows.push(json!({
            "id":case["id"],"expected":expected,"actual":actual,"matched":matched,
            "relation":format!("{:?}",result.relation),"direction":format!("{:?}",result.direction),
        }));
    }
    let report = json!({
        "model":model,"cases":rows.len(),"correct":correct,
        "accuracy":correct as f64 / rows.len().max(1) as f64,
        "rel":args[3],"phy":args[4],"results":rows
    });
    let out = PathBuf::from(&args[1]);
    if let Some(parent) = out.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(out, serde_json::to_vec_pretty(&report)?)?;
    println!("dream {}/{} with {}", correct, rows.len(), model);
    Ok(())
}

fn owner_context(
    id: MemoryId,
    rel: &mut Cva,
    phy: &mut Phylactery,
) -> Result<DreamMemoryContext, Box<dyn Error>> {
    if let Ok(value) = rel.dream_memory_context(id) {
        return Ok(value);
    }
    if let Ok(value) = phy.dream_memory_context(id) {
        return Ok(value);
    }
    Err(format!("Memory missing from both REL and PHY: {:?}", id).into())
}

fn memory_id(value: &str) -> Result<MemoryId, Box<dyn Error>> {
    if value.len() != 64 {
        return Err("invalid MemoryId width".into());
    }
    let mut bytes = [0_u8; 32];
    for (i, byte) in bytes.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&value[i * 2..i * 2 + 2], 16)?;
    }
    Ok(MemoryId(bytes))
}
