use std::process::{Command, Stdio};

use super::{Plugin, PluginFlowControl};

#[derive(Default)]
pub struct Math {}

impl Math {
    pub fn new() -> Self {
        Self::default()
    }
}
// TODO: async call, move inside textbox
impl Plugin for Math {
    fn search(&mut self, query: &str, ui: &mut eframe::egui::Ui) -> PluginFlowControl {
        if !query.starts_with('=') {
            return PluginFlowControl::Continue;
        }

        let call = Command::new("python")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .arg("-c")
            .arg(format!("from math import *\nprint({})", query.strip_prefix('=').unwrap()))
            .spawn();

        match call {
            Ok(child) => {
                let out = child.wait_with_output().unwrap();
                ui.label(String::from_utf8_lossy(&out.stdout));
                ui.label(String::from_utf8_lossy(&out.stderr));
            }
            Err(err) => {
                ui.label(format!("{:?}", err));
            }
        }
        PluginFlowControl::Break
    }
}
