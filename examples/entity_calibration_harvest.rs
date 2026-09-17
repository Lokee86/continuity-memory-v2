use reliquary_memory::{Cva, Memory, MemoryBodyId, MemoryId, Phylactery};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::collections::HashSet;
use std::error::Error;
use std::fs::{self, File};
use std::io::{BufWriter, Write};
use std::path::Path;

const SEED: &str = "insomnia-entity-v1";
const SHADOW_TARGET: usize = 256;
const OWNER_FLOOR: usize = 32;

#[derive(Clone)]
struct Case {
    owner_kind: &'static str,
    owner_id: String,
    memory_id: MemoryId,
    body_id: MemoryBodyId,
    memory: Memory,
    rank: [u8; 32],
}

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 4 {
        return Err(
            "usage: entity_calibration_harvest <project.rel> <user.phy> <output-dir>".into(),
        );
    }

    let mut rel = Cva::open(&args[1])?;
    let mut phy = Phylactery::open(&args[2])?;
    let rel_owner = rel.owner_id().ok_or("REL has no stable owner id")?;
    let phy_owner = phy.owner_id().ok_or("PHY has no stable owner id")?;

    let mut cases = Vec::new();
    for id in rel.memory_ids() {
        let body_id = rel.memory_body_id(id)?;
        let memory = rel.memory(id)?;
        cases.push(case("rel", &rel_owner, memory, body_id));
    }
    for id in phy.memory_ids() {
        let body_id = phy.memory_body_id(id)?;
        let memory = phy.memory(id)?;
        cases.push(case("phy", &phy_owner, memory, body_id));
    }
    cases.sort_by_key(|item| item.rank);

    let output_dir = Path::new(&args[3]);
    fs::create_dir_all(output_dir)?;
    write_candidates(&output_dir.join("candidates.jsonl"), &cases)?;
    write_shadow(&output_dir.join("shadow.jsonl"), &select_shadow(&cases))?;

    let rel_count = cases.iter().filter(|item| item.owner_kind == "rel").count();
    let phy_count = cases.len() - rel_count;
    println!(
        "harvested {} current Memories: {} REL, {} PHY",
        cases.len(),
        rel_count,
        phy_count
    );
    println!(
        "wrote {} deterministic shadow cases",
        SHADOW_TARGET.min(cases.len())
    );
    Ok(())
}

fn case(owner_kind: &'static str, owner_id: &str, memory: Memory, body_id: MemoryBodyId) -> Case {
    let mut hash = Sha256::new();
    hash.update(SEED.as_bytes());
    hash.update(b"\0");
    hash.update(owner_id.as_bytes());
    hash.update(b"\0");
    hash.update(memory.id.0);
    hash.update(b"\0");
    hash.update(body_id.0);
    Case {
        owner_kind,
        owner_id: owner_id.to_owned(),
        memory_id: memory.id,
        body_id,
        memory,
        rank: hash.finalize().into(),
    }
}

fn select_shadow(cases: &[Case]) -> Vec<Case> {
    let target = SHADOW_TARGET.min(cases.len());
    let floor = OWNER_FLOOR.min(target / 2);
    let mut selected = Vec::with_capacity(target);
    let mut keys = HashSet::new();

    for kind in ["rel", "phy"] {
        for item in cases
            .iter()
            .filter(|item| item.owner_kind == kind)
            .take(floor)
        {
            keys.insert(key(item));
            selected.push(item.clone());
        }
    }
    for item in cases {
        if selected.len() == target {
            break;
        }
        if keys.insert(key(item)) {
            selected.push(item.clone());
        }
    }
    selected.sort_by_key(|item| item.rank);
    selected
}

fn write_candidates(path: &Path, cases: &[Case]) -> Result<(), Box<dyn Error>> {
    let mut writer = BufWriter::new(File::create(path)?);
    for item in cases {
        let memory = &item.memory;
        writeln!(
            writer,
            "{}",
            json!({
                "owner_kind": item.owner_kind,
                "owner_id": item.owner_id,
                "memory_id": hex(&item.memory_id.0),
                "body_id": hex(&item.body_id.0),
                "rank": hex(&item.rank),
                "category": memory.category,
                "memory_type": memory.memory_type,
                "temporal_status": memory.temporal_status,
                "lifecycle_state": memory.lifecycle_state,
                "archived": memory.archived,
                "title": memory.title,
                "content": memory.content
            })
        )?;
    }
    Ok(())
}

fn write_shadow(path: &Path, cases: &[Case]) -> Result<(), Box<dyn Error>> {
    let mut writer = BufWriter::new(File::create(path)?);
    for item in cases {
        writeln!(
            writer,
            "{}",
            json!({
                "owner_kind": item.owner_kind,
                "owner_id": item.owner_id,
                "memory_id": hex(&item.memory_id.0),
                "body_id": hex(&item.body_id.0),
                "rank": hex(&item.rank),
                "tags": []
            })
        )?;
    }
    Ok(())
}

fn key(item: &Case) -> String {
    format!("{}:{}", item.owner_kind, hex(&item.memory_id.0))
}

fn hex(bytes: &[u8]) -> String {
    let mut text = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        use std::fmt::Write as _;
        write!(text, "{byte:02x}").expect("writing to String cannot fail");
    }
    text
}
