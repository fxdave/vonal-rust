use std::{
    env, fs,
    process::{Command, Stdio},
};

pub fn copy_to_clipboard(input: &str) {
    if is_program_in_path("xsel") {
        run("xsel", &["-b", "-i"], input);
    } else if is_program_in_path("xclip") {
        run("xclip", &["-selection", "c"], input);
    } else {
        println!("no xsel, and no xclip found")
    }
}

fn run(cmd: &str, args: &[&str], input: &str) {
    let echo = Command::new("echo")
        .arg("-n")
        .arg(input)
        .stdout(Stdio::piped())
        .spawn()
        .unwrap()
        .stdout
        .unwrap();
    let cmd = Command::new(cmd).args(args).stdin(echo).spawn().unwrap();
    cmd.wait_with_output().unwrap();
}

fn is_program_in_path(program: &str) -> bool {
    env::var("PATH")
        .unwrap_or("/".into())
        .split(":")
        .map(|dir| format!("{}/{}", dir, program))
        .filter_map(|path| fs::metadata(path).ok())
        .next()
        .is_some()
}
