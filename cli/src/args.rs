use crate::config_args::ConfigCommand;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(
    name = "continuity",
    version,
    about = "Repo-local Continuity bring-up CLI"
)]
pub struct Cli {
    #[arg(long, global = true, default_value = "continuity.cfg")]
    pub config: PathBuf,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    Cva {
        #[command(subcommand)]
        command: CvaCommand,
    },
    Import {
        #[command(subcommand)]
        command: ImportCommand,
    },
    Archive {
        #[command(subcommand)]
        command: ArchiveCommand,
    },
    Config {
        #[command(subcommand)]
        command: ConfigCommand,
    },
    Vectors {
        #[command(subcommand)]
        command: VectorsCommand,
    },
    Dev {
        #[command(subcommand)]
        command: DevCommand,
    },
}

#[derive(Subcommand, Debug)]
pub enum CvaCommand {
    Create { path: PathBuf },
    Info { path: PathBuf },
    Verify { path: PathBuf },
}

#[derive(Subcommand, Debug)]
pub enum ImportCommand {
    GraphJsonl {
        input: PathBuf,
        cva: PathBuf,
        #[arg(long, default_value_t = 8)]
        turns: usize,
        #[arg(long, default_value_t = 2)]
        overlap: usize,
        #[arg(long)]
        leave_tail_open: bool,
    },
}

#[derive(Subcommand, Debug)]
pub enum ArchiveCommand {
    Conversations {
        cva: PathBuf,
    },
    Show {
        cva: PathBuf,
        #[arg(long)]
        conversation: String,
        #[arg(long)]
        branch: String,
    },
    Fragments {
        cva: PathBuf,
        #[arg(long)]
        conversation: Option<String>,
    },
    Fragment {
        cva: PathBuf,
        id: String,
    },
}

#[derive(Subcommand, Debug)]
pub enum VectorsCommand {
    Status {
        cva: PathBuf,
    },
    Profiles {
        cva: PathBuf,
    },
    Probe {
        #[arg(required = true, trailing_var_arg = true)]
        text: Vec<String>,
    },
    Build {
        cva: PathBuf,
        #[arg(long, default_value_t = 16)]
        batch_size: usize,
        #[arg(long, default_value_t = 16)]
        concurrency: usize,
    },
}

#[derive(Subcommand, Debug)]
pub enum DevCommand {
    EstablishProfile {
        cva: PathBuf,
        #[arg(long)]
        dimensions: u32,
        #[arg(long, default_value = "l2")]
        normalization: String,
        #[arg(long)]
        seed: u64,
        #[arg(long, default_value_t = 0.0)]
        drift: f32,
    },
    BuildVectors {
        cva: PathBuf,
        #[arg(long)]
        profile: String,
        #[arg(long)]
        seed: u64,
        #[arg(long, default_value_t = 0.0)]
        drift: f32,
    },
    Search {
        cva: PathBuf,
        #[arg(long)]
        profile: String,
        #[arg(long)]
        seed: u64,
        #[arg(long, default_value_t = 0.0)]
        drift: f32,
        #[arg(long)]
        semantic_only: bool,
        #[arg(long, default_value_t = 10)]
        limit: usize,
        #[arg(required = true, trailing_var_arg = true)]
        query: Vec<String>,
    },
}
