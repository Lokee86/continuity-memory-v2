use crate::args::{Cli, Command, CvaCommand};
use crate::config_args::{ConfigCommand, CredentialCommand};
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
