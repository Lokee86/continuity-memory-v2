mod archive_cmd;
mod args;
#[cfg(test)]
mod args_tests;
mod config_args;
mod config_cmd;
mod cva_cmd;
mod dev_cmd;
mod import_cmd;
mod util;
mod vectors_cmd;

use anyhow::Result;
use args::{Cli, Command};
use clap::Parser;

fn main() {
    if let Err(error) = run() {
        eprintln!("error: {error:#}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Cva { command } => cva_cmd::run(command),
        Command::Import { command } => import_cmd::run(command),
        Command::Archive { command } => archive_cmd::run(command),
        Command::Config { command } => config_cmd::run(&cli.config, command),
        Command::Vectors { command } => vectors_cmd::run(command),
        Command::Dev { command } => dev_cmd::run(command),
    }
}
