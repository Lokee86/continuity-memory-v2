use reliquary_memory::{
    Cva, EntityId, MemoryEntityMentionKey, MemoryEntityResolutionStatus, Phylactery,
};
use std::collections::HashMap;
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let rel_path = args.next().expect("rel path");
    let phy_path = args.next().expect("phy path");
    let rel = Cva::open(Path::new(&rel_path))?;
    let phy = Phylactery::open(Path::new(&phy_path))?;

    println!("=== REL ===");
    inspect_rel(&rel)?;
    println!("=== PHY ===");
    inspect_phy(&phy)?;
    Ok(())
}

fn inspect_rel(owner: &Cva) -> Result<(), Box<dyn std::error::Error>> {
    let ids = owner.memory_ids();
    let routing = ids
        .iter()
        .filter(|id| owner.memory_routing_metadata(**id).is_some())
        .count();
    let mentions: usize = ids
        .iter()
        .filter_map(|id| owner.memory_routing_metadata(*id))
        .map(|m| m.entity_mentions.len())
        .sum();
    println!(
        "memories={} routing={} mentions={} entities={} states={}",
        ids.len(),
        routing,
        mentions,
        owner.entity_stats().entities,
        owner.entity_resolutions().len()
    );

    let mut counts = HashMap::<EntityId, usize>::new();
    for id in &ids {
        for entity_id in owner.entity_associations_for_memory(*id) {
            *counts.entry(entity_id).or_default() += 1;
        }
    }
    let entities = owner.entities();
    for entity in &entities {
        println!(
            "ENTITY\t{}\t{}\tassoc={}\t{}",
            entity.canonical_name,
            entity.kind,
            counts.get(&entity.id).copied().unwrap_or(0),
            entity.summary
        );
    }
    let names = entities
        .into_iter()
        .map(|e| (e.id, e.canonical_name))
        .collect();
    for state in owner.entity_resolutions() {
        print_state(mention_text_rel(owner, state.key)?, &state.status, &names);
    }
    Ok(())
}

fn inspect_phy(owner: &Phylactery) -> Result<(), Box<dyn std::error::Error>> {
    let ids = owner.memory_ids();
    let routing = ids
        .iter()
        .filter(|id| owner.memory_routing_metadata(**id).is_some())
        .count();
    let mentions: usize = ids
        .iter()
        .filter_map(|id| owner.memory_routing_metadata(*id))
        .map(|m| m.entity_mentions.len())
        .sum();
    println!(
        "memories={} routing={} mentions={} entities={} states={}",
        ids.len(),
        routing,
        mentions,
        owner.entity_stats().entities,
        owner.entity_resolutions().len()
    );

    let mut counts = HashMap::<EntityId, usize>::new();
    for id in &ids {
        for entity_id in owner.entity_associations_for_memory(*id) {
            *counts.entry(entity_id).or_default() += 1;
        }
    }
    for entity in owner.entities() {
        println!(
            "ENTITY\t{}\t{}\tassoc={}\t{}",
            entity.canonical_name,
            entity.kind,
            counts.get(&entity.id).copied().unwrap_or(0),
            entity.summary
        );
    }
    let names = owner
        .entities()
        .into_iter()
        .map(|e| (e.id, e.canonical_name))
        .collect();
    for state in owner.entity_resolutions() {
        print_state(mention_text_phy(owner, state.key)?, &state.status, &names);
    }
    Ok(())
}

fn mention_text_rel(
    owner: &Cva,
    key: MemoryEntityMentionKey,
) -> Result<String, Box<dyn std::error::Error>> {
    Ok(owner
        .memory_routing_metadata(key.memory_id)
        .and_then(|m| {
            m.entity_mentions.iter().find(|x| {
                x.field == key.field && x.start_byte == key.start_byte && x.end_byte == key.end_byte
            })
        })
        .map(|m| m.text.clone())
        .unwrap_or_else(|| "<missing mention>".to_owned()))
}

fn mention_text_phy(
    owner: &Phylactery,
    key: MemoryEntityMentionKey,
) -> Result<String, Box<dyn std::error::Error>> {
    Ok(owner
        .memory_routing_metadata(key.memory_id)
        .and_then(|m| {
            m.entity_mentions.iter().find(|x| {
                x.field == key.field && x.start_byte == key.start_byte && x.end_byte == key.end_byte
            })
        })
        .map(|m| m.text.clone())
        .unwrap_or_else(|| "<missing mention>".to_owned()))
}

fn print_state(
    mention: String,
    status: &MemoryEntityResolutionStatus,
    names: &HashMap<EntityId, String>,
) {
    match status {
        MemoryEntityResolutionStatus::Resolved { entity_id, reason } => println!(
            "STATE\t{}\tresolved\t{:?}\t{}",
            mention,
            reason,
            names
                .get(entity_id)
                .map(String::as_str)
                .unwrap_or("<missing entity>")
        ),
        MemoryEntityResolutionStatus::Rejected { reason } => {
            println!("STATE\t{}\trejected\t{:?}", mention, reason)
        }
        MemoryEntityResolutionStatus::Pending(value) => {
            println!("STATE\t{}\tpending\t{:?}", mention, value.reason)
        }
        MemoryEntityResolutionStatus::Dormant(value) => {
            println!("STATE\t{}\tdormant\t{:?}", mention, value.reason)
        }
    }
}
