use egui::{self, Ui};
use regex::Regex;
use std::process::Command;

use crate::{
    config::{ConfigBuilder, ConfigError},
    theme::list::{List, ListState},
};

use self::indexer::traits::AppIndex;

use super::{Plugin, PluginContext};

mod finder;
mod indexer;

#[derive(Default)]
pub struct Launcher {
    finder: finder::Finder,
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
    fn search(&mut self, ui: &mut Ui, ctx: &mut PluginContext) {
        if !ctx.query.starts_with(&self.config_prefix) {
            return;
        }
        let keywords = ctx.query.trim_start_matches(&self.config_prefix);
        let (keyword, args) = Self::split_query(keywords);
        let apps = self.find_apps(&keyword);
        let show_settings = keyword.starts_with(',');

        if show_settings {
            ui.add(List::new().with_builder(|list_ui| {
                list_ui.row(|ui| {
                    if ui.primary_action("Refresh application cache").activated {
                        self.reindex_apps()
                    }
                });
            }));

            return;
        }
        ui.add(List::new().with_builder(|list_ui| {
            for app in &apps {
                list_ui.row(|row_ui| {
                    row_ui.label("Launch");
                    if row_ui.primary_action(&app.name).activated {
                        self.run(&app.exec, &args);
                        ctx.query.clear();
                        ctx.gl_window.window().set_visible(false);
                        ListState::reset(row_ui.ui.ctx(), 0);
                    }

                    for action in &app.actions {
                        if row_ui.secondary_action(&action.name).activated {
                            self.run(&action.command, &args);
                            ctx.query.clear();
                            ctx.gl_window.window().set_visible(false);
                            ListState::reset(row_ui.ui.ctx(), 0);
                        }
                    }
                });
            }
        }));
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
