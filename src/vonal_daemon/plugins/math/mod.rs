use egui::{Color32, Ui};
use poll_promise::Promise;
use std::{
    process::{Command, Stdio},
    thread,
};

use super::{Plugin, PluginContext};
use crate::{
    config::{ConfigBuilder, ConfigError},
    theme::list::List,
    utils::clipboard::copy_to_clipboard,
};

#[derive(Default)]
pub struct Math {
    promise: Option<Promise<CommandResult>>,
    previous_query: String,
    result: Option<CommandResult>,
    config_prefix: String,
    config_python_header: String,
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
            config_prefix: Default::default(),
            config_python_header: Default::default(),
        }
    }
}
// TODO: async call, move inside textbox
impl Plugin for Math {
    fn search(&mut self, ui: &mut Ui, ctx: &mut PluginContext) {
        if !ctx.query.starts_with(&self.config_prefix) {
            return;
        }

        if self.previous_query != *ctx.query {
            self.promise = None;
        }

        let egui_ctx = ui.ctx().clone();
        let query_stripped = ctx
            .query
            .trim_start_matches(&self.config_prefix)
            .to_string();
        let config_python_header = self.config_python_header.clone();
        let promise = self.promise.get_or_insert_with(|| {
            let (sender, promise) = Promise::new();
            thread::spawn(move || {
                let call = Command::new("python")
                    .stdout(Stdio::piped())
                    .stderr(Stdio::piped())
                    .arg("-c")
                    .arg(format!(
                        "{}\nprint({})",
                        config_python_header, query_stripped
                    ))
                    .spawn();

                match call {
                    Ok(child) => {
                        let out = child.wait_with_output();
                        sender.send(match out {
                            Ok(out) => CommandResult {
                                stdout: String::from_utf8_lossy(&out.stdout).into(),
                                stderr: String::from_utf8_lossy(&out.stderr).into(),
                            },
                            Err(err) => CommandResult {
                                stdout: String::new(),
                                stderr: err.to_string(),
                            },
                        });
                        egui_ctx.request_repaint();
                    }
                    Err(err) => {
                        sender.send(CommandResult {
                            stdout: String::new(),
                            stderr: format!("{err:?}"),
                        });
                        egui_ctx.request_repaint();
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
        ui.add(
            List::new()
                .with_initally_selected_row(-1)
                .with_builder(|ui| {
                    ui.row(|ui| {
                        if ui.secondary_action("Copy").activated {
                            copy_to_clipboard(&stdout_to_copy)
                        }
                    });
                }),
        );

        self.previous_query = ctx.query.clone();
        ctx.break_flow();
    }

    fn before_search(&mut self, ctx: &mut PluginContext) {
        if ctx.query.starts_with('=') {
            ctx.break_flow();
        }
    }

    fn configure(&mut self, mut builder: ConfigBuilder) -> Result<ConfigBuilder, ConfigError> {
        builder.group("math_plugin", |builder| {
            self.config_prefix = builder.get_or_create("prefix", "=".to_string())?;
            self.config_python_header =
                builder.get_or_create("python_header", "from math import *".to_string())?;
            Ok(())
        })?;

        Ok(builder)
    }
}
