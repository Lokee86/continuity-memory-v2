use crate::config_args::ConfigCommand;
use clap::{Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(
    name = "reliquary",
    version,
    about = "Repo-local Reliquary bring-up CLI"
)]
pub struct Cli {
    #[arg(long, global = true, default_value = "reliquary.cfg")]
    pub config: PathBuf,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    #[command(name = "rel", visible_alias = "cva")]
    Rel {
        #[command(subcommand)]
        command: RelCommand,
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
    Insomnia {
        #[command(subcommand)]
        command: InsomniaCommand,
    },
    Dev {
        #[command(subcommand)]
        command: DevCommand,
    },
}

#[derive(Subcommand, Debug)]
pub enum RelCommand {
    Create {
        path: PathBuf,
        #[arg(long, value_enum, default_value_t = RelScopeArg::Project)]
        scope: RelScopeArg,
    },
    Info {
        path: PathBuf,
    },
    Verify {
        path: PathBuf,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
pub enum RelScopeArg {
    Organization,
    Project,
    Connection,
}

#[derive(Subcommand, Debug)]
pub enum ImportCommand {
    GraphJsonl {
        input: PathBuf,
        #[arg(value_name = "REL")]
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
        #[arg(value_name = "REL")]
        cva: PathBuf,
    },
    Show {
        #[arg(value_name = "REL")]
        cva: PathBuf,
        #[arg(long)]
        conversation: String,
        #[arg(long)]
        branch: String,
    },
    Fragments {
        #[arg(value_name = "REL")]
        cva: PathBuf,
        #[arg(long)]
        conversation: Option<String>,
    },
    Fragment {
        #[arg(value_name = "REL")]
        cva: PathBuf,
        id: String,
    },
}

#[derive(Subcommand, Debug)]
pub enum VectorsCommand {
    Status {
        #[arg(value_name = "REL")]
        cva: PathBuf,
    },
    Profiles {
        #[arg(value_name = "REL")]
        cva: PathBuf,
    },
    Probe {
        #[arg(required = true, trailing_var_arg = true)]
        text: Vec<String>,
    },
    Build {
        #[arg(value_name = "REL")]
        cva: PathBuf,
        #[arg(long, default_value_t = 16)]
        batch_size: usize,
        #[arg(long, default_value_t = 16)]
        concurrency: usize,
    },
}

#[derive(Subcommand, Debug)]
pub enum InsomniaCommand {
    Run {
        #[arg(value_name = "REL")]
        cva: PathBuf,
        #[arg(long, default_value_t = reliquary_memory::DEFAULT_INSOMNIA_WORKERS)]
        workers: usize,
        #[arg(long, default_value = "private")]
        scope: String,
        #[arg(long, default_value_t = 16)]
        embedding_batch_size: usize,
        #[arg(long, default_value_t = 16)]
        embedding_concurrency: usize,
        #[arg(long)]
        existing_queue_only: bool,
    },
}

#[derive(Subcommand, Debug)]
pub enum DevCommand {
    EstablishProfile {
        #[arg(value_name = "REL")]
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
        #[arg(value_name = "REL")]
        cva: PathBuf,
        #[arg(long)]
        profile: String,
        #[arg(long)]
        seed: u64,
        #[arg(long, default_value_t = 0.0)]
        drift: f32,
    },
    Search {
        #[arg(value_name = "REL")]
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
