use common::socket_path;
use std::{env, io::Write, os::unix::net::UnixStream, process::ExitCode};

use crate::common::{CommandParseResult, Commands};

#[path = "../common.rs"]
mod common;

fn main() -> ExitCode {
    // `args` returns the arguments passed to the program
    let args: Vec<String> = env::args().collect();

    // First argument is the message to be sent
    let commands: Commands = match args.as_slice()[1..]
        .iter()
        .map(String::as_str)
        .collect::<CommandParseResult>()
        .0
    {
        Ok(commands) => commands,
        Err(error) => {
            eprintln!("vonalc: could not parse arguments: {error:?}");
            return ExitCode::FAILURE;
        }
    };

    let socket = socket_path();

    // Connect to socket
    let mut stream = match UnixStream::connect(&socket) {
        Ok(stream) => stream,
        Err(error) => {
            eprintln!(
                "vonalc: cannot reach vonal on {}: {error}.\n\
                 Is the daemon running? Start it first.",
                socket.display()
            );
            return ExitCode::FAILURE;
        }
    };

    let command_binary = match bincode::serialize(&commands) {
        Ok(bytes) => bytes,
        Err(error) => {
            eprintln!("vonalc: could not encode command: {error}");
            return ExitCode::FAILURE;
        }
    };

    // Send message
    if let Err(error) = stream.write_all(&command_binary) {
        eprintln!("vonalc: couldn't send message: {error}");
        return ExitCode::FAILURE;
    }

    ExitCode::SUCCESS
}
