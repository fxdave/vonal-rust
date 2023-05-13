use serde::{Deserialize, Serialize};

pub const SOCKET_PATH: &str = "vonal-socket";

#[derive(Debug, Serialize, Deserialize)]
pub enum Command {
    Show,
    Hide,
    Toggle,
    SetQuery { query: String },
}

fn parse_command(
    command: &str,
    tokens: &mut dyn Iterator<Item = &str>,
) -> Result<Command, CommandParseError> {
    match command {
        "show" => Ok(Command::Show),
        "hide" => Ok(Command::Hide),
        "toggle" => Ok(Command::Toggle),
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
    EmptyArgument
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
        let commands = commands.and_then(|vector| Ok(Commands(vector)));
        CommandParseResult(commands)
    }
}
