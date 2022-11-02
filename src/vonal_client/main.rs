#![feature(panic_info_message)]

use common::SOCKET_PATH;
use std::{env, io::Write, os::unix::net::UnixStream, path::Path};

#[path = "../common.rs"]
mod common;

fn main() {
    // Set less distracting panic message
    std::panic::set_hook(Box::new(|info| match info.message() {
        Some(message) => println!("Error: {}", message),
        None => println!("{}", info),
    }));

    // `args` returns the arguments passed to the program
    let args: Vec<String> = env::args().collect();
    let socket = Path::new(SOCKET_PATH);

    // First argument is the message to be sent
    let message = args.as_slice()[1..].join(",");

    // Connect to socket
    let mut stream =
        UnixStream::connect(&socket).expect("Vonal is not running. You have to start it first.");

    // Send message
    assert!(
        stream.write_all(message.as_bytes()).is_ok(),
        "couldn't send message"
    );
}
