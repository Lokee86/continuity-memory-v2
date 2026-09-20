use reliquary_memory::{
    Cva, Entity, EntityDraft, EntityId, MemoryEntityResolution, MemoryEntityResolutionStatus,
    MemoryId, MemoryRoutingMetadata, Phylactery,
};
use std::collections::HashSet;
use std::error::Error;
use std::fs;
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn Error>> {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    if args.len() != 5 {
        return Err("usage: entity_backfill_project14 <processed28-rel> <processed28-phy> <source14-rel> <source14-phy> <output-dir>".into());
    }
    let output = PathBuf::from(&args[4]);
    fs::create_dir_all(&output)?;
    let rel_path = output.join("project.prj.rel");
    let phy_path = output.join("user.phy");
    fs::copy(&args[2], &rel_path)?;
    fs::copy(&args[3], &phy_path)?;
    make_writable(&rel_path)?;
    make_writable(&phy_path)?;

    let mut source_rel = Cva::open(&args[0])?;
    let source_phy = Phylactery::open(&args[1])?;
    let mut target_rel = Cva::open(&rel_path)?;
    let mut target_phy = Phylactery::open(&phy_path)?;

    let rel = project_rel(&mut source_rel, &mut target_rel)?;
    let phy = project_phy(&source_phy, &mut target_phy)?;
    target_rel.sync()?;
    target_phy.sync()?;

    println!(
        "projected rel memories={} metadata={} entities={} resolutions={} associations={}",
        rel.memories, rel.metadata, rel.entities, rel.resolutions, rel.associations
    );
    println!(
        "projected phy memories={} metadata={} entities={} resolutions={} associations={}",
        phy.memories, phy.metadata, phy.entities, phy.resolutions, phy.associations
    );
    Ok(())
}

fn make_writable(path: &PathBuf) -> Result<(), Box<dyn Error>> {
    let mut permissions = fs::metadata(path)?.permissions();
    permissions.set_readonly(false);
    fs::set_permissions(path, permissions)?;
    Ok(())
}

#[derive(Default)]
struct Stats {
    memories: usize,
    metadata: usize,
    entities: usize,
    resolutions: usize,
    associations: usize,
}

fn project_rel(source: &mut Cva, target: &mut Cva) -> Result<Stats, Box<dyn Error>> {
    let source_ids = source.memory_ids().into_iter().collect::<HashSet<_>>();
    let target_ids = target.memory_ids();
    assert_subset(&source_ids, &target_ids, "REL")?;

    let mut stats = Stats {
        memories: target_ids.len(),
        ..Stats::default()
    };
    let mut resolutions = Vec::new();
    let mut associations = Vec::new();
    let mut required_entities = HashSet::new();

    for id in target_ids {
        if let Some(metadata) = source.memory_routing_metadata(id).cloned() {
            let body_id = target.memory_body_id(id)?;
            target.put_memory_routing_metadata(MemoryRoutingMetadata {
                memory_id: id,
                body_id,
                entity_mentions: metadata.entity_mentions,
            })?;
            stats.metadata += 1;
        }
        let records = source.entity_resolutions_for_memory(id);
        collect_entities(&records, &mut required_entities);
        resolutions.extend(records);
        for entity_id in source.entity_associations_for_memory(id) {
            required_entities.insert(entity_id);
            associations.push((id, entity_id));
        }
    }

    copy_rel_entities(source, target, &required_entities)?;
    stats.entities = required_entities.len();
    for record in resolutions {
        target.import_entity_resolution(record)?;
        stats.resolutions += 1;
    }
    for (memory_id, entity_id) in associations {
        let version = target.graph_version();
        target.set_entity_association(memory_id, entity_id, true, version)?;
        stats.associations += 1;
    }
    Ok(stats)
}

fn project_phy(source: &Phylactery, target: &mut Phylactery) -> Result<Stats, Box<dyn Error>> {
    let source_ids = source.memory_ids().into_iter().collect::<HashSet<_>>();
    let target_ids = target.memory_ids();
    assert_subset(&source_ids, &target_ids, "PHY")?;

    let mut stats = Stats {
        memories: target_ids.len(),
        ..Stats::default()
    };
    let mut resolutions = Vec::new();
    let mut associations = Vec::new();
    let mut required_entities = HashSet::new();

    for id in target_ids {
        if let Some(metadata) = source.memory_routing_metadata(id).cloned() {
            let body_id = target.memory_body_id(id)?;
            target.put_memory_routing_metadata(MemoryRoutingMetadata {
                memory_id: id,
                body_id,
                entity_mentions: metadata.entity_mentions,
            })?;
            stats.metadata += 1;
        }
        let records = source.entity_resolutions_for_memory(id);
        collect_entities(&records, &mut required_entities);
        resolutions.extend(records);
        for entity_id in source.entity_associations_for_memory(id) {
            required_entities.insert(entity_id);
            associations.push((id, entity_id));
        }
    }

    copy_phy_entities(source, target, &required_entities)?;
    stats.entities = required_entities.len();
    for record in resolutions {
        target.import_entity_resolution(record)?;
        stats.resolutions += 1;
    }
    for (memory_id, entity_id) in associations {
        let version = target.graph_version();
        target.set_entity_association(memory_id, entity_id, true, version)?;
        stats.associations += 1;
    }
    Ok(stats)
}

fn assert_subset(
    source: &HashSet<MemoryId>,
    target: &[MemoryId],
    owner: &str,
) -> Result<(), Box<dyn Error>> {
    let missing = target.iter().filter(|id| !source.contains(id)).count();
    if missing != 0 {
        return Err(format!(
            "{owner} 14-day snapshot has {missing} Memories absent from 28-day snapshot"
        )
        .into());
    }
    Ok(())
}

fn collect_entities(records: &[MemoryEntityResolution], output: &mut HashSet<EntityId>) {
    for record in records {
        match &record.status {
            MemoryEntityResolutionStatus::Resolved { entity_id, .. } => {
                output.insert(*entity_id);
            }
            MemoryEntityResolutionStatus::Pending(value) => {
                output.extend(value.candidate_entity_ids.iter().copied());
            }
            MemoryEntityResolutionStatus::Dormant(value) => {
                output.extend(value.candidate_entity_ids.iter().copied());
            }
            MemoryEntityResolutionStatus::Rejected { .. } => {}
        }
    }
}

fn copy_rel_entities(
    source: &Cva,
    target: &mut Cva,
    ids: &HashSet<EntityId>,
) -> Result<(), Box<dyn Error>> {
    let mut ids = ids.iter().copied().collect::<Vec<_>>();
    ids.sort();
    for id in ids {
        let entity = source.entity(id)?;
        target.publish_entity(Some(id), 0, draft(&entity))?;
    }
    Ok(())
}

fn copy_phy_entities(
    source: &Phylactery,
    target: &mut Phylactery,
    ids: &HashSet<EntityId>,
) -> Result<(), Box<dyn Error>> {
    let mut ids = ids.iter().copied().collect::<Vec<_>>();
    ids.sort();
    for id in ids {
        let entity = source.entity(id)?;
        target.publish_entity(Some(id), 0, draft(&entity))?;
    }
    Ok(())
}

fn draft(entity: &Entity) -> EntityDraft {
    EntityDraft {
        canonical_name: entity.canonical_name.clone(),
        aliases: entity.aliases.clone(),
        kind: entity.kind.clone(),
        summary: entity.summary.clone(),
        mutation_id: entity.mutation_id.clone(),
        created_at_ns: entity.created_at_ns,
        updated_at_ns: entity.updated_at_ns,
    }
}
