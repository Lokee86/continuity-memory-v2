use crate::args::{Cli, Command, InsomniaCommand, RelCommand, RelScopeArg, VectorsCommand};
use crate::config_args::{ConfigCommand, CredentialCommand, ModelCommand, ReasoningArg};
use clap::Parser;

#[test]
fn parses_rel_command() {
    let cli = Cli::try_parse_from(["reliquary", "rel", "info", "sample.prj.rel"]).unwrap();
    assert!(matches!(
        cli.command,
        Command::Rel {
            command: RelCommand::Info { .. }
        }
    ));
}

#[test]
fn legacy_cva_command_remains_an_alias() {
    let cli = Cli::try_parse_from(["reliquary", "cva", "info", "sample.cva"]).unwrap();
    assert!(matches!(
        cli.command,
        Command::Rel {
            command: RelCommand::Info { .. }
        }
    ));
}

#[test]
fn rel_create_accepts_scope_kind() {
    let cli = Cli::try_parse_from([
        "reliquary",
        "rel",
        "create",
        "acme.org.rel",
        "--scope",
        "organization",
    ])
    .unwrap();
    assert!(matches!(
        cli.command,
        Command::Rel {
            command: RelCommand::Create {
                scope: RelScopeArg::Organization,
                ..
            }
        }
    ));
}

#[test]
fn api_key_command_accepts_stdin_mode_without_secret_argument() {
    let cli = Cli::try_parse_from([
        "reliquary",
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
        "reliquary",
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
fn codex_device_login_command_accepts_credential_id() {
    let cli =
        Cli::try_parse_from(["reliquary", "config", "credential", "login-codex", "codex"]).unwrap();
    assert!(matches!(
        cli.command,
        Command::Config {
            command: ConfigCommand::Credential {
                command: CredentialCommand::LoginCodex { .. }
            }
        }
    ));
}

#[test]
fn insomnia_model_route_is_configurable() {
    let cli = Cli::try_parse_from([
        "reliquary",
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
fn dream_model_route_is_configurable() {
    let cli = Cli::try_parse_from([
        "continuity",
        "config",
        "model",
        "set-dream",
        "--provider",
        "openai-codex",
        "--model",
        "gpt-5.6-luna",
        "--credential",
        "codex",
        "--reasoning",
        "low",
    ])
    .unwrap();
    assert!(matches!(
        cli.command,
        Command::Config {
            command: ConfigCommand::Model {
                command: ModelCommand::SetDream { .. }
            }
        }
    ));
}

#[test]
fn codex_general_route_accepts_low_reasoning() {
    let cli = Cli::try_parse_from([
        "reliquary",
        "config",
        "model",
        "set-general",
        "--provider",
        "openai-codex",
        "--model",
        "gpt-5.6-luna",
        "--credential",
        "codex",
        "--reasoning",
        "low",
    ])
    .unwrap();
    assert!(matches!(
        cli.command,
        Command::Config {
            command: ConfigCommand::Model {
                command: ModelCommand::SetGeneral {
                    reasoning: Some(ReasoningArg::Low),
                    ..
                }
            }
        }
    ));
}

#[test]
fn live_vector_probe_accepts_free_text() {
    let cli = Cli::try_parse_from(["reliquary", "vectors", "probe", "hello", "world"]).unwrap();
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
        "reliquary",
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
    let cli = Cli::try_parse_from(["reliquary", "vectors", "build", "sample.cva"]).unwrap();
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

#[test]
fn insomnia_run_exposes_worker_concurrency() {
    let cli = Cli::try_parse_from([
        "reliquary",
        "insomnia",
        "run",
        "sample.cva",
        "--workers",
        "8",
        "--embedding-concurrency",
        "4",
    ])
    .unwrap();
    assert!(matches!(
        cli.command,
        Command::Insomnia {
            command: InsomniaCommand::Run {
                workers: 8,
                embedding_concurrency: 4,
                ..
            }
        }
    ));
}
