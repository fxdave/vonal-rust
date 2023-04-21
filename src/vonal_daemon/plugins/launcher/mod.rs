use egui::{self, Ui};
use regex::Regex;
use std::process::Command;

use crate::{
    config::{ConfigBuilder, ConfigError},
    theme::list::{CreateList, ListState},
    GlutinWindowContext,
};

use self::indexer::traits::AppIndex;

use super::{Plugin, PluginFlowControl, Preparation};

mod finder;
mod indexer;

#[derive(Default)]
pub struct Launcher {
    finder: finder::Finder,
    list: ListState,
    config_prefix: String,
    config_index_path: bool,
    config_number_of_results: usize,
}

impl Launcher {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn reindex_apps(&mut self) {
        self.finder = self.index_apps_and_get_finder()
    }

    pub fn run(&self, command: &str, args: &str) -> Option<()> {
        let switches_regex = Regex::new(r"%(f|F|u|U|i|c|k|v|m|d|D|n|N)").unwrap();
        let mut new_command = switches_regex.replace_all(command, args).to_string();
        if new_command == command {
            new_command = format!("{command} {args}");
        }
        Command::new("/bin/sh")
            .arg("-c")
            .arg(new_command)
            .spawn()
            .ok()?;
        Some(())
    }

    fn index_apps_and_get_finder(&self) -> finder::Finder {
        let indices = indexer::Indexer::default().index(self.config_index_path);
        finder::Finder::new(indices)
    }

    fn find_apps(&self, query: &str) -> Vec<AppIndex> {
        self.finder
            .find(query, self.config_number_of_results)
            .into_iter()
            .map(|app_match| app_match.index)
            .cloned()
            .collect()
    }

    fn split_query(query: &str) -> (String, String) {
        let query_sections = query.split(' ').collect::<Vec<_>>();

        if let Some((keyword, args)) = query_sections.split_first() {
            let keyword = keyword.to_string();
            let args = args.join(" ");
            return (keyword, args);
        }

        (String::new(), String::new())
    }
}

impl Plugin for Launcher {
    fn search(
        &mut self,
        query: &mut String,
        ui: &mut Ui,
        gl_window: &GlutinWindowContext,
    ) -> PluginFlowControl {
        if !query.starts_with(&self.config_prefix) {
            return PluginFlowControl::Continue;
        }
        let keywords = query.trim_start_matches(&self.config_prefix);
        let (keyword, args) = Self::split_query(keywords);
        let apps = self.find_apps(&keyword);
        let show_settings = keyword.starts_with(',');

        if show_settings {
            self.list.update(ui.ctx(), 1, |_| 1);
            ui.list(self.list, |mut ui| {
                ui.row(|mut ui| {
                    if ui.primary_action("Refresh application cache").activated {
                        self.reindex_apps()
                    }
                });
            });

            return PluginFlowControl::Continue;
        }

        self.list.update(ui.ctx(), apps.len(), |idx| {
            apps[idx].actions.len() + 1 // 1 primary action
        });

        ui.list(self.list, |mut ui| {
            for app in &apps {
                ui.row(|mut ui| {
                    ui.label("Launch");
                    if ui.primary_action(&app.name).activated {
                        self.run(&app.exec, &args);
                        query.clear();
                        gl_window.window().set_visible(false);
                    }

                    for action in &app.actions {
                        if ui.secondary_action(&action.name).activated {
                            self.run(&action.command, &args);
                            query.clear();
                            gl_window.window().set_visible(false);
                        }
                    }
                })
            }
        });

        PluginFlowControl::Continue
    }

    #[allow(clippy::useless_let_if_seq)]
    fn before_search(
        &mut self,
        _query: &mut String,
        ctx: &egui::Context,
        _gl_window: &GlutinWindowContext,
    ) -> Preparation {
        let preparation = self.list.before_search(ctx);
        Preparation {
            disable_cursor: preparation.disable_cursor,
            plugin_flow_control: PluginFlowControl::Continue,
        }
    }

    fn configure(&mut self, mut builder: ConfigBuilder) -> Result<ConfigBuilder, ConfigError> {
        builder.group("launcher_plugin", |builder| {
            self.config_prefix = builder.get_or_create("prefix", "".to_string())?;
            self.config_index_path = builder.get_or_create("index_path", true)?;
            self.config_number_of_results = builder.get_or_create("number_of_results", 7)?;
            self.reindex_apps();
            Ok(())
        })?;
        Ok(builder)
    }
}
