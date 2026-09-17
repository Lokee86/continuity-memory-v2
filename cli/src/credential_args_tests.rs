use crate::args::{Cli, Command};
use crate::config_args::{ConfigCommand, CredentialCommand};
use clap::Parser;

#[test]
fn codex_refresh_command_accepts_credential_id() {
    let cli = Cli::try_parse_from([
        "reliquary",
        "config",
        "credential",
        "refresh-codex",
        "codex",
    ])
    .unwrap();
    assert!(matches!(
        cli.command,
        Command::Config {
            command: ConfigCommand::Credential {
                command: CredentialCommand::RefreshCodex { .. }
            }
        }
    ));
}
