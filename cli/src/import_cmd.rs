use crate::args::ImportCommand;
use anyhow::{Context, Result, anyhow};
use continuity_memory::{Branch, Cva, FragmentConfig, IncomingAttachment, IncomingTurn};
use serde::Deserialize;
use std::fs::{self, File};
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

#[derive(Deserialize)]
struct InputAttachment {
    path: PathBuf,
    filename: Option<String>,
    mime_type: Option<String>,
}

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
        #[serde(default)]
        attachments: Vec<InputAttachment>,
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
            let input_dir = input.parent().unwrap_or_else(|| Path::new("."));
            let mut nodes = 0_usize;
            let mut attachments = 0_usize;
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
                        attachments: incoming_attachments,
                    } => {
                        let incoming_attachments = incoming_attachments
                            .into_iter()
                            .map(|attachment| load_attachment(input_dir, attachment))
                            .collect::<Result<Vec<_>>>()?;
                        attachments += incoming_attachments.len();
                        archive.ingest_turn(IncomingTurn {
                            id,
                            conversation_id,
                            parent_id,
                            role,
                            timestamp_ns,
                            content,
                            attachments: incoming_attachments,
                        })?;
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
                "imported: nodes={} attachments={} branches={} new_fragments={} cva={}",
                nodes,
                attachments,
                branches,
                fragments,
                cva.display()
            );
        }
    }
    Ok(())
}

fn load_attachment(input_dir: &Path, attachment: InputAttachment) -> Result<IncomingAttachment> {
    let path = if attachment.path.is_absolute() {
        attachment.path
    } else {
        input_dir.join(attachment.path)
    };
    let filename = match attachment.filename {
        Some(filename) => filename,
        None => path
            .file_name()
            .and_then(|value| value.to_str())
            .map(str::to_owned)
            .ok_or_else(|| anyhow!("attachment path has no UTF-8 filename: {}", path.display()))?,
    };
    let bytes =
        fs::read(&path).with_context(|| format!("failed to read attachment {}", path.display()))?;
    Ok(IncomingAttachment {
        filename,
        mime_type: attachment.mime_type,
        bytes,
    })
}
