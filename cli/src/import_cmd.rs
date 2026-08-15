use crate::args::ImportCommand;
use anyhow::{Result, anyhow};
use continuity_memory::{Branch, Cva, FragmentConfig};
use serde::Deserialize;
use std::fs::File;
use std::io::{BufRead, BufReader};

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
    },
}

pub fn run(command: ImportCommand) -> Result<()> {
    match command {
        ImportCommand::GraphJsonl {
            input,
            cva,
            turns,
            overlap,
            leave_tail_open,
        } => {
            if turns == 0 || overlap >= turns {
                return Err(anyhow!(
                    "fragment policy requires turns > 0 and overlap < turns"
                ));
            }
            let mut archive = if cva.exists() {
                Cva::open(&cva)?
            } else {
                Cva::create(&cva)?
            };
            let config = FragmentConfig { turns, overlap };
            let mut nodes = 0_usize;
            let mut branches = 0_usize;
            let mut fragments = 0_usize;
            for line in BufReader::new(File::open(&input)?).lines() {
                let line = line?;
                if line.trim().is_empty() {
                    continue;
                }
                match serde_json::from_str::<Input>(&line)? {
                    Input::Node {
                        id,
                        conversation_id,
                        parent_id,
                        role,
                        timestamp_ns,
                        content,
                    } => {
                        archive.append_node(
                            id,
                            conversation_id,
                            parent_id,
                            role,
                            timestamp_ns,
                            &content,
                        )?;
                        nodes += 1;
                    }
                    Input::Branch {
                        id,
                        conversation_id,
                        leaf_node_id,
                        canonical,
                    } => {
                        archive.append_branch(Branch {
                            id: id.clone(),
                            conversation_id: conversation_id.clone(),
                            leaf_node_id,
                            canonical,
                        })?;
                        fragments += archive
                            .materialize_branch_fragments(
                                &conversation_id,
                                &id,
                                config,
                                !leave_tail_open,
                            )?
                            .len();
                        branches += 1;
                    }
                }
            }
            archive.sync()?;
            println!(
                "imported: nodes={} branches={} new_fragments={} cva={}",
                nodes,
                branches,
                fragments,
                cva.display()
            );
        }
    }
    Ok(())
}
