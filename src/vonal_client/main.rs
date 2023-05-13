use common::SOCKET_PATH;
use std::{env, io::Write, os::unix::net::UnixStream, path::Path};

use crate::common::{CommandParseResult, Commands};

#[path = "../common.rs"]
mod common;

fn main() {
    // `args` returns the arguments passed to the program
    let args: Vec<String> = env::args().collect();
    let socket = Path::new(SOCKET_PATH);

    // First argument is the message to be sent
    let commands: Commands = args.as_slice()[1..]
        .iter()
        .map(String::as_str)
        .collect::<CommandParseResult>()
        .0
        .unwrap();

    // Connect to socket
    let mut stream =
        UnixStream::connect(socket).expect("Vonal is not running. You have to start it first.");

    let command_binary = bincode::serialize(&commands).unwrap();

    // Send message
    assert!(
        stream.write_all(&command_binary).is_ok(),
        "couldn't send message"
    );
}
