use reliquary_memory::{
    ConfiguredGeneralEndpoint, Cva, EntityCandidateConfig, EntityResolver, MemoryEntityMention,
    MemoryEntityMentionKey, MemoryId, MemoryRoutingMetadata, MemoryTextField, ModelSwitchboard,
    Phylactery, ReliquaryConfig,
};
use serde_json::Value;
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

fn main() -> Result<(), Box<dyn Error>> {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    if args.len() < 5 {
        return Err("usage: entity_backfill_resolve <extraction-results.jsonl> <source-rel> <source-phy> <output-dir> <config-path> [rounds]".into());
    }
    let rounds = args
        .get(5)
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(4)
        .max(1);
    let output = PathBuf::from(&args[3]);
    fs::create_dir_all(&output)?;
    let rel_path = output.join("project.prj.rel");
    let phy_path = output.join("user.phy");
    copy_once(Path::new(&args[1]), &rel_path)?;
    copy_once(Path::new(&args[2]), &phy_path)?;

    let mut rel = Cva::open(&rel_path)?;
    let mut phy = Phylactery::open(&phy_path)?;
    let rows = load_jsonl(&args[0])?;
    install_metadata(&mut rel, &mut phy, &rows)?;
    rel.sync()?;
    phy.sync()?;

    let config = ReliquaryConfig::open(Path::new(&args[4]))?;
    let switchboard = ModelSwitchboard::new(config.models, config.credentials)?;
    let endpoint = ConfiguredGeneralEndpoint::from_entity_resolution_switchboard(&switchboard)?;
    println!(
        "entity_resolution model={}",
        reliquary_memory::GeneralEndpoint::model(&endpoint)
    );

    resolve_rel(&mut rel, endpoint.clone(), rounds)?;
    resolve_phy(&mut phy, endpoint, rounds)?;
    rel.sync()?;
    phy.sync()?;
    println!(
        "done rel_entities={} phy_entities={} rel_resolutions={} phy_resolutions={}",
        rel.entity_stats().entities,
        phy.entity_stats().entities,
        rel.entity_resolutions().len(),
        phy.entity_resolutions().len()
    );
    Ok(())
}

fn copy_once(source: &Path, target: &Path) -> Result<(), Box<dyn Error>> {
    if !target.exists() {
        fs::copy(source, target)?;
    }
    make_writable(target)?;
    Ok(())
}

fn make_writable(path: &Path) -> Result<(), Box<dyn Error>> {
    let mut permissions = fs::metadata(path)?.permissions();
    permissions.set_readonly(false);
    fs::set_permissions(path, permissions)?;
    Ok(())
}

fn load_jsonl(path: impl AsRef<Path>) -> Result<Vec<Value>, Box<dyn Error>> {
    Ok(fs::read_to_string(path)?
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(serde_json::from_str)
        .collect::<Result<_, _>>()?)
}

fn install_metadata(
    rel: &mut Cva,
    phy: &mut Phylactery,
    rows: &[Value],
) -> Result<(), Box<dyn Error>> {
    for row in rows {
        let id = parse_memory_id(row["memory_id"].as_str().ok_or("missing memory_id")?)?;
        let mentions = parse_mentions(&row["actual_entity_mentions"])?;
        match row["owner_kind"].as_str().ok_or("missing owner_kind")? {
            "rel" => {
                let body_id = rel.memory_body_id(id)?;
                rel.put_memory_routing_metadata(MemoryRoutingMetadata {
                    memory_id: id,
                    body_id,
                    entity_mentions: mentions,
                })?;
            }
            "phy" => {
                let body_id = phy.memory_body_id(id)?;
                phy.put_memory_routing_metadata(MemoryRoutingMetadata {
                    memory_id: id,
                    body_id,
                    entity_mentions: mentions,
                })?;
            }
            other => return Err(format!("unknown owner_kind {other}").into()),
        }
    }
    Ok(())
}

fn parse_mentions(value: &Value) -> Result<Vec<MemoryEntityMention>, Box<dyn Error>> {
    value
        .as_array()
        .ok_or("mentions must be array")?
        .iter()
        .map(|m| {
            Ok(MemoryEntityMention {
                field: match m["field"].as_str().ok_or("mention field missing")? {
                    "title" => MemoryTextField::Title,
                    "content" => MemoryTextField::Content,
                    other => return Err(format!("unknown mention field {other}").into()),
                },
                start_byte: m["start_byte"].as_u64().ok_or("start_byte missing")? as u32,
                end_byte: m["end_byte"].as_u64().ok_or("end_byte missing")? as u32,
                text: m["text"].as_str().ok_or("mention text missing")?.to_owned(),
            })
        })
        .collect()
}

fn parse_memory_id(value: &str) -> Result<MemoryId, Box<dyn Error>> {
    if value.len() != 64 {
        return Err("invalid Memory ID length".into());
    }
    let mut bytes = [0_u8; 32];
    for (i, byte) in bytes.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&value[i * 2..i * 2 + 2], 16)?;
    }
    Ok(MemoryId(bytes))
}

fn rel_keys(owner: &mut Cva) -> Result<Vec<MemoryEntityMentionKey>, Box<dyn Error>> {
    let ids = owner.memory_ids();
    let mut values = Vec::new();
    for id in ids {
        let Some(metadata) = owner.memory_routing_metadata(id).cloned() else {
            continue;
        };
        let memory = owner.memory(id)?;
        let time = memory.source_time_ns.unwrap_or(memory.created_at_ns);
        for mention in metadata.entity_mentions {
            values.push((
                time,
                id.0,
                field_ord(mention.field),
                mention.start_byte,
                MemoryEntityMentionKey::new(id, &mention),
            ));
        }
    }
    values.sort_by_key(|v| (v.0, v.1, v.2, v.3));
    Ok(values.into_iter().map(|v| v.4).collect())
}

fn phy_keys(owner: &mut Phylactery) -> Result<Vec<MemoryEntityMentionKey>, Box<dyn Error>> {
    let ids = owner.memory_ids();
    let mut values = Vec::new();
    for id in ids {
        let Some(metadata) = owner.memory_routing_metadata(id).cloned() else {
            continue;
        };
        let memory = owner.memory(id)?;
        let time = memory.source_time_ns.unwrap_or(memory.created_at_ns);
        for mention in metadata.entity_mentions {
            values.push((
                time,
                id.0,
                field_ord(mention.field),
                mention.start_byte,
                MemoryEntityMentionKey::new(id, &mention),
            ));
        }
    }
    values.sort_by_key(|v| (v.0, v.1, v.2, v.3));
    Ok(values.into_iter().map(|v| v.4).collect())
}

fn resolve_rel(
    owner: &mut Cva,
    endpoint: ConfiguredGeneralEndpoint,
    rounds: usize,
) -> Result<(), Box<dyn Error>> {
    let reconciliation_endpoint = endpoint.clone();
    let resolver = EntityResolver::new(endpoint);
    let keys = rel_keys(owner)?;
    for round in 1..=rounds {
        let mut changes = 0usize;
        for (index, key) in keys.iter().copied().enumerate() {
            let outcome = owner.resolve_entity_mention(
                &resolver,
                key,
                EntityCandidateConfig::default(),
                now_ns(),
            )?;
            changes += usize::from(
                outcome.entity_created || outcome.association_changed || outcome.resolution_changed,
            );
            if index % 50 == 49 {
                owner.sync()?;
            }
        }
        owner.sync()?;
        println!(
            "rel round={round} mentions={} changes={changes}",
            keys.len()
        );
        print_reconciliation(
            "REL",
            &owner.reconcile_entities(&reconciliation_endpoint, now_ns())?,
        );
        if changes == 0 {
            break;
        }
    }
    Ok(())
}

fn resolve_phy(
    owner: &mut Phylactery,
    endpoint: ConfiguredGeneralEndpoint,
    rounds: usize,
) -> Result<(), Box<dyn Error>> {
    let reconciliation_endpoint = endpoint.clone();
    let resolver = EntityResolver::new(endpoint);
    let keys = phy_keys(owner)?;
    for round in 1..=rounds {
        let mut changes = 0usize;
        for (index, key) in keys.iter().copied().enumerate() {
            let outcome = owner.resolve_entity_mention(
                &resolver,
                key,
                EntityCandidateConfig::default(),
                now_ns(),
            )?;
            changes += usize::from(
                outcome.entity_created || outcome.association_changed || outcome.resolution_changed,
            );
            if index % 50 == 49 {
                owner.sync()?;
            }
        }
        owner.sync()?;
        println!(
            "phy round={round} mentions={} changes={changes}",
            keys.len()
        );
        print_reconciliation(
            "PHY",
            &owner.reconcile_entities(&reconciliation_endpoint, now_ns())?,
        );
        if changes == 0 {
            break;
        }
    }
    Ok(())
}

fn field_ord(field: MemoryTextField) -> u8 {
    match field {
        MemoryTextField::Title => 0,
        MemoryTextField::Content => 1,
    }
}

fn print_reconciliation(label: &str, report: &reliquary_memory::EntityReconciliationReport) {
    println!(
        "entity reconciliation: owner={label} rounds={} candidates={} deterministic_merges={} model_merges={} rejected_reconsidered={} unresolved_pairs={} clean={} findings={}",
        report.rounds,
        report.candidate_pairs,
        report.deterministic_merges,
        report.model_merges,
        report.rejected_mentions_reconsidered,
        report.unresolved_pairs,
        report.final_audit.is_clean(),
        report.final_audit.finding_count(),
    );
}

fn now_ns() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos().min(i64::MAX as u128) as i64)
        .unwrap_or(0)
}
