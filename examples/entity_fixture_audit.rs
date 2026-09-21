use reliquary_memory::{
    Cva, EntityId, MemoryEntityMentionKey, MemoryEntityResolutionStatus, Phylactery,
};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::Path;

#[derive(Clone)]
struct AuditEntity {
    id: EntityId,
    name: String,
    aliases: Vec<String>,
    kind: String,
    summary: String,
    degree: usize,
}

#[derive(Clone)]
struct AuditState {
    mention: String,
    status: &'static str,
    reason: String,
    target: Option<EntityId>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let rel_path = args.next().expect("rel path");
    let phy_path = args.next().expect("phy path");
    let rel = Cva::open(Path::new(&rel_path))?;
    let phy = Phylactery::open(Path::new(&phy_path))?;

    let (rel_entities, rel_states, rel_assoc) = collect_rel(&rel)?;
    let (phy_entities, phy_states, phy_assoc) = collect_phy(&phy)?;

    audit("REL", &rel_entities, &rel_states, rel_assoc);
    audit("PHY", &phy_entities, &phy_states, phy_assoc);
    Ok(())
}

fn collect_rel(
    owner: &Cva,
) -> Result<(Vec<AuditEntity>, Vec<AuditState>, usize), Box<dyn std::error::Error>> {
    let ids = owner.memory_ids();
    let mut degree = HashMap::<EntityId, usize>::new();
    let mut associations = 0usize;
    for id in &ids {
        for entity_id in owner.entity_associations_for_memory(*id) {
            associations += 1;
            *degree.entry(entity_id).or_default() += 1;
        }
    }
    let entities = owner
        .entities()
        .into_iter()
        .map(|entity| AuditEntity {
            degree: degree.get(&entity.id).copied().unwrap_or(0),
            id: entity.id,
            name: entity.canonical_name,
            aliases: entity.aliases,
            kind: entity.kind,
            summary: entity.summary,
        })
        .collect::<Vec<_>>();
    let names = entities
        .iter()
        .map(|entity| (entity.id, entity.name.clone()))
        .collect::<HashMap<_, _>>();
    let states = owner
        .entity_resolutions()
        .into_iter()
        .map(|state| {
            let mention = mention_text_rel(owner, state.key);
            audit_state(mention, &state.status, &names)
        })
        .collect();
    Ok((entities, states, associations))
}

fn collect_phy(
    owner: &Phylactery,
) -> Result<(Vec<AuditEntity>, Vec<AuditState>, usize), Box<dyn std::error::Error>> {
    let ids = owner.memory_ids();
    let mut degree = HashMap::<EntityId, usize>::new();
    let mut associations = 0usize;
    for id in &ids {
        for entity_id in owner.entity_associations_for_memory(*id) {
            associations += 1;
            *degree.entry(entity_id).or_default() += 1;
        }
    }
    let entities = owner
        .entities()
        .into_iter()
        .map(|entity| AuditEntity {
            degree: degree.get(&entity.id).copied().unwrap_or(0),
            id: entity.id,
            name: entity.canonical_name,
            aliases: entity.aliases,
            kind: entity.kind,
            summary: entity.summary,
        })
        .collect::<Vec<_>>();
    let names = entities
        .iter()
        .map(|entity| (entity.id, entity.name.clone()))
        .collect::<HashMap<_, _>>();
    let states = owner
        .entity_resolutions()
        .into_iter()
        .map(|state| {
            let mention = mention_text_phy(owner, state.key);
            audit_state(mention, &state.status, &names)
        })
        .collect();
    Ok((entities, states, associations))
}

fn audit(label: &str, entities: &[AuditEntity], states: &[AuditState], associations: usize) {
    println!("=== {label} ===");
    let mut status = BTreeMap::<String, usize>::new();
    let mut reason = BTreeMap::<String, usize>::new();
    let mut recurrence = BTreeMap::<String, usize>::new();
    let mut safety_reasons = BTreeMap::<String, BTreeMap<String, usize>>::new();
    for state in states {
        *status.entry(state.status.into()).or_default() += 1;
        *reason.entry(state.reason.clone()).or_default() += 1;
        if state.reason == "RecurrenceRequired" {
            *recurrence.entry(state.status.into()).or_default() += 1;
        }
        if matches!(
            state.reason.as_str(),
            "WrapperCategory"
                | "TransientValue"
                | "GenericRole"
                | "AbstractProcess"
                | "SentenceLocal"
        ) {
            *safety_reasons
                .entry(state.reason.clone())
                .or_default()
                .entry(state.status.into())
                .or_default() += 1;
        }
    }
    let singletons = entities.iter().filter(|entity| entity.degree == 1).count();
    let zero_degree = entities.iter().filter(|entity| entity.degree == 0).count();
    let degree_ge3 = entities.iter().filter(|entity| entity.degree >= 3).count();
    println!(
        "SUMMARY states={} entities={} associations={} assoc_per_entity={:.3} singleton={} singleton_pct={:.2} zero_degree={} degree_ge3={} degree_ge3_pct={:.2}",
        states.len(),
        entities.len(),
        associations,
        if entities.is_empty() {
            0.0
        } else {
            associations as f64 / entities.len() as f64
        },
        singletons,
        pct(singletons, entities.len()),
        zero_degree,
        degree_ge3,
        pct(degree_ge3, entities.len()),
    );
    println!("STATUS {:?}", status);
    println!("REASONS {:?}", reason);
    println!("RECURRENCE {:?}", recurrence);
    println!("SAFETY_REASON_STATUS {:?}", safety_reasons);

    print_clusters("EXACT_SAME_SURFACE_KIND", exact_clusters(entities));
    print_clusters("NORMALIZED_SURFACE_KIND", normalized_clusters(entities));
    print_clusters("ALIAS_KEY_KIND", alias_clusters(entities));
    print_clusters(
        "CROSS_KIND_ALIAS_SHADOW",
        cross_kind_alias_shadow_clusters(entities),
    );

    let python = entities
        .iter()
        .filter(|entity| entity.name.eq_ignore_ascii_case("Python"))
        .collect::<Vec<_>>();
    println!("PYTHON_ENTITIES count={}", python.len());
    for entity in python {
        println!(
            "  {} | {} | degree={} | {:?}",
            entity.name, entity.kind, entity.degree, entity.aliases
        );
    }

    println!("ZERO_DEGREE_ENTITIES");
    for entity in entities.iter().filter(|entity| entity.degree == 0) {
        println!("  {} | {} | {:?}", entity.name, entity.kind, entity.aliases);
    }

    println!("TRANSIENT_LIKE_ENTITIES");
    for entity in entities.iter().filter(|entity| transient_like(entity)) {
        println!(
            "  {} | {} | degree={} | {}",
            entity.name, entity.kind, entity.degree, entity.summary
        );
    }

    println!("WRAPPER_LIKE_ENTITIES");
    for entity in entities.iter().filter(|entity| {
        let name = entity.name.to_ascii_lowercase();
        let summary = entity.summary.to_ascii_lowercase();
        name.contains("generative-ai")
            || name.contains("generative ai")
            || summary.contains("generative-ai category")
            || summary.contains("generative ai category")
    }) {
        println!(
            "  {} | {} | degree={} | aliases={:?} | {}",
            entity.name, entity.kind, entity.degree, entity.aliases, entity.summary
        );
    }

    println!("SINGLETON_OTHER_ENTITIES");
    for entity in entities
        .iter()
        .filter(|entity| entity.degree == 1 && entity.kind == "other")
    {
        println!("  {} | {}", entity.name, entity.summary);
    }

    println!("DEVTOOLS_ENTITIES");
    for entity in entities.iter().filter(|entity| {
        entity.name.to_ascii_lowercase().contains("devtools")
            || entity
                .aliases
                .iter()
                .any(|alias| alias.to_ascii_lowercase().contains("devtools"))
    }) {
        println!(
            "  {} | {} | degree={} | aliases={:?} | {}",
            entity.name, entity.kind, entity.degree, entity.aliases, entity.summary
        );
    }

    println!("KNOWN_CASE_STATES");
    let needles = [
        "python",
        "gds",
        "generative",
        "background_music_player",
        "backgroundmusic",
        "client_connection_service.gd",
        "clientconnectionservice",
        "v2 ui",
        "clientv2",
        "uuid",
        "github.com",
        "radial",
        "torpedo",
        "devtools",
        "space rocks repository",
    ];
    let targets = entities
        .iter()
        .map(|entity| (entity.id, format!("{} : {}", entity.name, entity.kind)))
        .collect::<HashMap<_, _>>();
    for state in states.iter().filter(|state| {
        let mention = state.mention.to_ascii_lowercase();
        needles.iter().any(|needle| mention.contains(needle))
    }) {
        let target = state
            .target
            .and_then(|id| targets.get(&id))
            .map(String::as_str)
            .unwrap_or("-");
        println!(
            "  {:?} | {} | {} | {}",
            state.mention, state.status, state.reason, target
        );
    }

    println!("TOP_DEGREE");
    let mut top = entities.iter().collect::<Vec<_>>();
    top.sort_by(|a, b| b.degree.cmp(&a.degree).then_with(|| a.name.cmp(&b.name)));
    for entity in top.into_iter().take(20) {
        println!(
            "  {} | {} | degree={}",
            entity.name, entity.kind, entity.degree
        );
    }
}

fn audit_state(
    mention: String,
    status: &MemoryEntityResolutionStatus,
    names: &HashMap<EntityId, String>,
) -> AuditState {
    match status {
        MemoryEntityResolutionStatus::Resolved { entity_id, reason } => AuditState {
            mention,
            status: "resolved",
            reason: format!("{reason:?}"),
            target: names.contains_key(entity_id).then_some(*entity_id),
        },
        MemoryEntityResolutionStatus::Rejected { reason } => AuditState {
            mention,
            status: "rejected",
            reason: format!("{reason:?}"),
            target: None,
        },
        MemoryEntityResolutionStatus::Pending(value) => AuditState {
            mention,
            status: "pending",
            reason: format!("{:?}", value.reason),
            target: None,
        },
        MemoryEntityResolutionStatus::Dormant(value) => AuditState {
            mention,
            status: "dormant",
            reason: format!("{:?}", value.reason),
            target: None,
        },
    }
}

fn exact_clusters(entities: &[AuditEntity]) -> Vec<(String, Vec<String>)> {
    let mut map = BTreeMap::<(String, String), BTreeSet<String>>::new();
    for entity in entities {
        map.entry((entity.name.trim().to_lowercase(), entity.kind.clone()))
            .or_default()
            .insert(entity_label(entity));
    }
    map.into_iter()
        .filter(|(_, names)| names.len() > 1)
        .map(|((surface, kind), names)| {
            (format!("{surface} : {kind}"), names.into_iter().collect())
        })
        .collect()
}

fn normalized_clusters(entities: &[AuditEntity]) -> Vec<(String, Vec<String>)> {
    cluster_by(entities, |surface| vec![normalized_surface(surface)])
}

fn alias_clusters(entities: &[AuditEntity]) -> Vec<(String, Vec<String>)> {
    cluster_by(entities, alias_keys)
}

fn cluster_by<F>(entities: &[AuditEntity], keys: F) -> Vec<(String, Vec<String>)>
where
    F: Fn(&str) -> Vec<String>,
{
    let mut map = BTreeMap::<(String, String), BTreeSet<String>>::new();
    for entity in entities {
        for surface in
            std::iter::once(entity.name.as_str()).chain(entity.aliases.iter().map(String::as_str))
        {
            for key in keys(surface).into_iter().filter(|key| !key.is_empty()) {
                map.entry((key, entity.kind.clone()))
                    .or_default()
                    .insert(entity_label(entity));
            }
        }
    }
    map.into_iter()
        .filter(|(_, names)| names.len() > 1)
        .map(|((surface, kind), names)| {
            (format!("{surface} : {kind}"), names.into_iter().collect())
        })
        .collect()
}

fn cross_kind_alias_shadow_clusters(entities: &[AuditEntity]) -> Vec<(String, Vec<String>)> {
    let mut map = BTreeMap::<String, Vec<(EntityId, String, String, bool)>>::new();
    for entity in entities {
        for (surface, is_alias) in std::iter::once((entity.name.as_str(), false))
            .chain(entity.aliases.iter().map(|alias| (alias.as_str(), true)))
        {
            let key = surface.trim().to_lowercase();
            if key.is_empty() {
                continue;
            }
            let values = map.entry(key).or_default();
            if let Some(existing) = values.iter_mut().find(|value| value.0 == entity.id) {
                existing.3 |= is_alias;
            } else {
                values.push((
                    entity.id,
                    entity.name.clone(),
                    entity.kind.clone(),
                    is_alias,
                ));
            }
        }
    }

    map.into_iter()
        .filter_map(|(surface, mut values)| {
            if values.len() < 2 || !values.iter().any(|value| value.3) {
                return None;
            }
            let kinds = values
                .iter()
                .map(|value| value.2.as_str())
                .collect::<BTreeSet<_>>();
            if kinds.len() < 2 {
                return None;
            }
            values.sort_by(|left, right| {
                left.1
                    .cmp(&right.1)
                    .then_with(|| left.2.cmp(&right.2))
                    .then_with(|| left.0.cmp(&right.0))
            });
            Some((
                surface,
                values
                    .into_iter()
                    .map(|(_, name, kind, is_alias)| {
                        format!(
                            "{name} : {kind} ({})",
                            if is_alias { "alias" } else { "canonical" }
                        )
                    })
                    .collect(),
            ))
        })
        .collect()
}

fn print_clusters(label: &str, clusters: Vec<(String, Vec<String>)>) {
    println!("{label} count={}", clusters.len());
    for (key, names) in clusters {
        println!("  {} => {:?}", key, names);
    }
}

fn normalized_surface(value: &str) -> String {
    value
        .chars()
        .flat_map(char::to_lowercase)
        .filter(|value| value.is_alphanumeric())
        .collect()
}

fn alias_keys(value: &str) -> Vec<String> {
    let trimmed = value.trim();
    let lower = trimmed.to_ascii_lowercase();
    let mut keys = Vec::new();
    if lower.starts_with("http://") || lower.starts_with("https://") {
        if let Some(repo) = trimmed
            .split_once("://")
            .and_then(|(_, rest)| rest.split('/').filter(|segment| !segment.is_empty()).last())
        {
            keys.push(format!(
                "repository:{}",
                normalized_surface(repo.trim_end_matches(".git"))
            ));
        }
    } else if lower.ends_with(" repository") || lower.ends_with(" repo") {
        let suffix = if lower.ends_with(" repository") {
            " repository"
        } else {
            " repo"
        };
        let base = trimmed[..trimmed.len() - suffix.len()]
            .trim()
            .trim_start_matches('@');
        keys.push(format!("repository:{}", normalized_surface(base)));
    }
    let plural_base = trimmed.strip_suffix('s').unwrap_or(trimmed);
    if plural_base.len() >= 2
        && plural_base
            .chars()
            .all(|value| value.is_ascii_uppercase() || value.is_ascii_digit())
    {
        keys.push(format!("acronym:{}", plural_base.to_ascii_lowercase()));
    }
    keys
}

fn entity_label(entity: &AuditEntity) -> String {
    format!("{}#{}", entity.name, short_id(entity.id))
}

fn short_id(id: EntityId) -> String {
    id.0.iter()
        .take(4)
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn transient_like(entity: &AuditEntity) -> bool {
    let name = entity.name.trim();
    let lower = name.to_ascii_lowercase();
    lower == "clientv2"
        || lower.starts_with("commit ")
        || ((7..=40).contains(&name.len()) && name.chars().all(|value| value.is_ascii_hexdigit()))
}

fn mention_text_rel(owner: &Cva, key: MemoryEntityMentionKey) -> String {
    owner
        .memory_routing_metadata(key.memory_id)
        .and_then(|metadata| {
            metadata.entity_mentions.iter().find(|mention| {
                mention.field == key.field
                    && mention.start_byte == key.start_byte
                    && mention.end_byte == key.end_byte
            })
        })
        .map(|mention| mention.text.clone())
        .unwrap_or_else(|| "<missing mention>".into())
}

fn mention_text_phy(owner: &Phylactery, key: MemoryEntityMentionKey) -> String {
    owner
        .memory_routing_metadata(key.memory_id)
        .and_then(|metadata| {
            metadata.entity_mentions.iter().find(|mention| {
                mention.field == key.field
                    && mention.start_byte == key.start_byte
                    && mention.end_byte == key.end_byte
            })
        })
        .map(|mention| mention.text.clone())
        .unwrap_or_else(|| "<missing mention>".into())
}

fn pct(value: usize, total: usize) -> f64 {
    if total == 0 {
        0.0
    } else {
        value as f64 * 100.0 / total as f64
    }
}
