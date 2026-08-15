use continuity_memory::{Branch, Cva, FragmentConfig};
use serde::Deserialize;
use std::collections::HashMap;
use std::env;
use std::fs::{self, File};
use std::io::{BufRead, BufReader};
use std::path::PathBuf;

#[derive(Deserialize)]
#[serde(tag = "kind")]
enum Input {
    #[serde(rename = "node")]
    Node {
        id: String,
        conversation_id: String,
        parent_id: Option<String>,
        role: String,
        timestamp_ns: i64,
        content: String,
    },
    #[serde(rename = "branch")]
    Branch {
        id: String,
        conversation_id: String,
        leaf_node_id: String,
        canonical: bool,
        path: Vec<String>,
    },
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 3 {
        return Err("usage: archive_roundtrip <graph.jsonl> <output.cva>".into());
    }
    let input = PathBuf::from(&args[1]);
    let output = PathBuf::from(&args[2]);
    if output.exists() {
        fs::remove_file(&output)?;
    }

    let mut archive = Cva::create(&output)?;
    let mut expected_paths: Vec<(String, String, Vec<String>)> = Vec::new();
    let mut expected_content = HashMap::new();
    for line in BufReader::new(File::open(input)?).lines() {
        match serde_json::from_str::<Input>(&line?)? {
            Input::Node {
                id,
                conversation_id,
                parent_id,
                role,
                timestamp_ns,
                content,
            } => {
                expected_content.insert((conversation_id.clone(), id.clone()), content.clone());
                archive.append_node(
                    id,
                    conversation_id,
                    parent_id,
                    role,
                    timestamp_ns,
                    &content,
                )?;
            }
            Input::Branch {
                id,
                conversation_id,
                leaf_node_id,
                canonical,
                path,
            } => {
                archive.append_branch(Branch {
                    id: id.clone(),
                    conversation_id: conversation_id.clone(),
                    leaf_node_id,
                    canonical,
                })?;
                archive.materialize_branch_fragments(
                    &conversation_id,
                    &id,
                    FragmentConfig::default(),
                    true,
                )?;
                expected_paths.push((conversation_id, id, path));
            }
        }
    }
    archive.sync()?;
    let before = archive.stats();
    drop(archive);

    let mut reopened = Cva::open(&output)?;
    if reopened.stats() != before {
        return Err("archive stats changed after reopen".into());
    }
    let mut expanded = 0_usize;
    for (conversation_id, branch_id, expected) in &expected_paths {
        let turns = reopened.branch_turns(conversation_id, branch_id)?;
        let actual: Vec<_> = turns.iter().map(|turn| turn.node_id.clone()).collect();
        if &actual != expected {
            return Err(format!("branch {branch_id} path mismatch").into());
        }
        for turn in &turns {
            if expected_content.get(&(conversation_id.clone(), turn.node_id.clone()))
                != Some(&turn.content)
            {
                return Err(format!("node {} content mismatch", turn.node_id).into());
            }
        }
        expanded += turns.len();
    }

    println!(
        "roundtrip ok: {} nodes, {} branches, {} fragments, {} content objects, {} expanded refs, {} bytes",
        before.nodes,
        before.branches,
        before.fragments,
        before.content_objects,
        expanded,
        fs::metadata(output)?.len()
    );
    Ok(())
}
