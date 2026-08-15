use crate::args::{Cli, Command, CvaCommand, VectorsCommand};
use crate::config_args::{ConfigCommand, CredentialCommand, ModelCommand};
use clap::Parser;

#[test]
fn parses_repo_local_cva_command() {
    let cli = Cli::try_parse_from(["continuity", "cva", "info", "sample.cva"]).unwrap();
    assert!(matches!(
        cli.command,
        Command::Cva {
            command: CvaCommand::Info { .. }
        }
    ));
}

#[test]
fn api_key_command_accepts_stdin_mode_without_secret_argument() {
    let cli = Cli::try_parse_from([
        "continuity",
        "config",
        "credential",
        "add-api-key",
        "ready",
        "--stdin",
    ])
    .unwrap();
    assert!(matches!(
        cli.command,
        Command::Config {
            command: ConfigCommand::Credential {
                command: CredentialCommand::AddApiKey { stdin: true, .. }
            }
        }
    ));
}

#[test]
fn api_key_command_rejects_direct_secret_option() {
    let result = Cli::try_parse_from([
        "continuity",
        "config",
        "credential",
        "add-api-key",
        "ready",
        "--api-key",
        "do-not-accept",
    ]);
    assert!(result.is_err());
}

#[test]
fn insomnia_model_route_is_configurable() {
    let cli = Cli::try_parse_from([
        "continuity",
        "config",
        "model",
        "set-insomnia",
        "--provider",
        "openai-ready",
        "--model",
        "memory-model",
        "--credential",
        "insomnia",
        "--url",
        "https://example.test/v1/chat/completions",
    ])
    .unwrap();
    assert!(matches!(
        cli.command,
        Command::Config {
            command: ConfigCommand::Model {
                command: ModelCommand::SetInsomnia { .. }
            }
        }
    ));
}

#[test]
fn live_vector_probe_accepts_free_text() {
    let cli = Cli::try_parse_from(["continuity", "vectors", "probe", "hello", "world"]).unwrap();
    assert!(matches!(
        cli.command,
        Command::Vectors {
            command: VectorsCommand::Probe { .. }
        }
    ));
}

#[test]
fn live_vector_build_accepts_concurrency_controls() {
    let cli = Cli::try_parse_from([
        "continuity",
        "vectors",
        "build",
        "sample.cva",
        "--batch-size",
        "32",
        "--concurrency",
        "8",
    ])
    .unwrap();
    assert!(matches!(
        cli.command,
        Command::Vectors {
            command: VectorsCommand::Build {
                batch_size: 32,
                concurrency: 8,
                ..
            }
        }
    ));
}

#[test]
fn live_vector_build_uses_measured_defaults() {
    let cli = Cli::try_parse_from(["continuity", "vectors", "build", "sample.cva"]).unwrap();
    assert!(matches!(
        cli.command,
        Command::Vectors {
            command: VectorsCommand::Build {
                batch_size: 16,
                concurrency: 16,
                ..
            }
        }
    ));
}
