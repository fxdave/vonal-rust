use egui::{self, Ui};
use regex::Regex;
use std::process::Command;

use crate::{
    theme::list::{CreateList, ListState},
    GlutinWindowContext,
};

use self::indexer::traits::{AppIndex, IndexApps};

use super::{Plugin, PluginFlowControl, Preparation};

mod finder;
mod indexer;

pub struct Launcher {
    finder: finder::Finder,
    list: ListState,
}

impl Launcher {
    pub fn new() -> Self {
        Self {
            finder: Self::index_apps_and_get_finder(),
            list: Default::default(),
        }
    }

    pub fn reindex_apps(&mut self) {
        self.finder = Self::index_apps_and_get_finder()
    }

    fn index_apps_and_get_finder() -> finder::Finder {
        let indices = indexer::Indexer::default().index();
        finder::Finder::new(indices)
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

    fn find_apps(&self, query: &str) -> Vec<AppIndex> {
        self.finder
            .find(query)
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
        let (keyword, args) = Self::split_query(query);
        let apps = self.find_apps(&keyword);

        self.list.update(ui.ctx(), apps.len(), |idx| {
            apps[idx].actions.len() + 1 // 1 primary action
        });

        ui.list(self.list, |mut ui| {
            if keyword.starts_with(',') {
                // Show plugin settings
                ui.row(|mut ui| {
                    if ui.primary_action("Refresh application cache").activated {
                        self.reindex_apps()
                    }
                });
                return;
            }

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
        let disable_cursor = self.list.before_search(ctx);
        Preparation {
            disable_cursor,
            plugin_flow_control: PluginFlowControl::Continue,
        }
    }
}
