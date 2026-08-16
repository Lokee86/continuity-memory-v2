use continuity_memory::Cva;
use serde_json::json;
use std::collections::HashSet;
use std::env;
use std::fs;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = env::args().skip(1);
    let cva_path = args
        .next()
        .ok_or("usage: dump_insomnia_memories <cva> <output.json>")?;
    let output = args
        .next()
        .ok_or("usage: dump_insomnia_memories <cva> <output.json>")?;
    let mut cva = Cva::open(&cva_path)?;
    let mut seen = HashSet::new();
    let mut rows = Vec::new();
    for episode in cva.episodes() {
        for attempt in cva.insomnia_attempts(episode.id) {
            for id in attempt.memory_ids {
                if !seen.insert(id) {
                    continue;
                }
                let memory = cva.memory(id)?;
                rows.push(json!({
                    "category": memory.category,
                    "type": memory.memory_type,
                    "title": memory.title,
                    "content": memory.content,
                    "source_node_id": memory.source_node_id,
                    "content_source_conversation_id": memory.content_source_conversation_id,
                    "content_source_node_id": memory.content_source_node_id,
                    "grounding_source_conversation_id": memory.grounding_source_conversation_id,
                    "grounding_source_node_id": memory.grounding_source_node_id,
                }));
            }
        }
    }
    rows.sort_by(|a, b| {
        a["source_node_id"]
            .as_str()
            .cmp(&b["source_node_id"].as_str())
            .then_with(|| a["title"].as_str().cmp(&b["title"].as_str()))
    });
    fs::write(output, serde_json::to_string_pretty(&rows)?)?;
    println!("dumped {} memories", rows.len());
    Ok(())
}
