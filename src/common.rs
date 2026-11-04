use serde::{Deserialize, Serialize};

use std::path::PathBuf;

/// Absolute path to the IPC socket.
///
/// This must not be a relative path: the daemon and `vonalc` are generally
/// started from different working directories, so a relative path made
/// `vonalc` silently connect to the wrong place (or nothing at all) unless it
/// happened to be invoked from the daemon's cwd.
pub fn socket_path() -> PathBuf {
    match std::env::var_os("XDG_RUNTIME_DIR") {
        Some(dir) if !dir.is_empty() => PathBuf::from(dir).join("vonal.sock"),
        _ => {
            let user = std::env::var("USER").unwrap_or_else(|_| "default".into());
            PathBuf::from(format!("/tmp/vonal-{user}.sock"))
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub enum Command {
    Show,
    Hide,
    Toggle,
    SetQuery { query: String },
    Restart,
}

fn parse_command(
    command: &str,
    tokens: &mut dyn Iterator<Item = &str>,
) -> Result<Command, CommandParseError> {
    match command {
        "show" => Ok(Command::Show),
        "hide" => Ok(Command::Hide),
        "toggle" => Ok(Command::Toggle),
        "restart" => Ok(Command::Restart),
        "set_query" => Ok(Command::SetQuery {
            query: tokens
                .next()
                .ok_or(CommandParseError::EmptyArgument)?
                .to_string(),
        }),
        command => Err(CommandParseError::UnknownCommand {
            command: command.into(),
        }),
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Commands(pub Vec<Command>);

#[derive(Debug)]
pub enum CommandParseError {
    UnknownCommand { command: String },
    EmptyArgument,
}

pub struct CommandParseResult(pub Result<Commands, CommandParseError>);

impl<'a> FromIterator<&'a str> for CommandParseResult {
    fn from_iter<T: IntoIterator<Item = &'a str>>(iter: T) -> Self {
        let mut tokens = iter.into_iter();
        let mut commands = Vec::new();
        while let Some(command) = tokens.next() {
            let parsed_command = parse_command(command, &mut tokens);
            commands.push(parsed_command);
        }

        let commands: Result<Vec<Command>, CommandParseError> = commands.into_iter().collect();
        let commands = commands.map(Commands);
        CommandParseResult(commands)
    }
}
