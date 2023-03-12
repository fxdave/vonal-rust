use egui::Color32;
use poll_promise::Promise;
use std::{
    process::{Command, Stdio},
    thread,
};

use super::{Plugin, PluginFlowControl};
use crate::{
    theme::list::{CreateList, ListState},
    utils::clipboard::copy_to_clipboard,
    GlutinWindowContext,
};

#[derive(Default)]
pub struct Math {
    promise: Option<Promise<CommandResult>>,
    previous_query: String,
    result: Option<CommandResult>,
    list_state: ListState,
}

#[derive(Clone)]
struct CommandResult {
    stdout: String,
    stderr: String,
}

impl Math {
    pub fn new() -> Self {
        Self {
            promise: Default::default(),
            previous_query: Default::default(),
            result: Default::default(),
            list_state: ListState::new(-1),
        }
    }
}
// TODO: async call, move inside textbox
impl Plugin for Math {
    fn search(
        &mut self,
        query: &mut String,
        ui: &mut egui::Ui,
        _: &GlutinWindowContext,
    ) -> PluginFlowControl {
        if !query.starts_with('=') {
            return PluginFlowControl::Continue;
        }

        if self.previous_query != *query {
            self.promise = None;
        }

        let ctx = ui.ctx().clone();
        let query_stripped = query.strip_prefix('=').unwrap().to_string();
        let promise = self.promise.get_or_insert_with(|| {
            let (sender, promise) = Promise::new();
            thread::spawn(move || {
                let call = Command::new("python")
                    .stdout(Stdio::piped())
                    .stderr(Stdio::piped())
                    .arg("-c")
                    .arg(format!("from math import *\nprint({query_stripped})"))
                    .spawn();

                match call {
                    Ok(child) => {
                        let out = child.wait_with_output().unwrap();
                        sender.send(CommandResult {
                            stdout: String::from_utf8_lossy(&out.stdout).into(),
                            stderr: String::from_utf8_lossy(&out.stderr).into(),
                        });
                        ctx.request_repaint();
                    }
                    Err(err) => {
                        sender.send(CommandResult {
                            stdout: String::new(),
                            stderr: format!("{err:?}"),
                        });
                        ctx.request_repaint();
                    }
                }
            });

            promise
        });

        if let Some(result) = promise.ready() {
            self.result = Some(result.clone());
        }

        if let Some(result) = &self.result {
            if !result.stdout.is_empty() {
                ui.colored_label(Color32::from_white_alpha(255), result.stdout.clone());
            }
            if !result.stderr.is_empty() {
                ui.colored_label(Color32::from_white_alpha(64), result.stderr.clone());
            }
        }

        let stdout_to_copy = self
            .result
            .as_ref()
            .map(|x| x.stdout.to_string())
            .unwrap_or_default();
        self.list_state.update(ui.ctx(), 1, |_x| 1);
        ui.list(self.list_state, |mut ui| {
            ui.row(|mut ui| {
                if ui.secondary_action("Copy").activated {
                    copy_to_clipboard(&stdout_to_copy)
                }
            });
        });

        self.previous_query = query.clone();
        PluginFlowControl::Break
    }

    fn before_search(
        &mut self,
        query: &mut String,
        ctx: &egui::Context,
        _: &GlutinWindowContext,
    ) -> super::Preparation {
        let preparation = self.list_state.before_search(ctx);

        super::Preparation {
            disable_cursor: preparation.disable_cursor,
            plugin_flow_control: if query.starts_with('=') {
                PluginFlowControl::Break
            } else {
                PluginFlowControl::Continue
            },
        }
    }
}
