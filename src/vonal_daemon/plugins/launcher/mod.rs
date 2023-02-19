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
    results: Vec<AppIndex>,
    list_state: ListState,
}

impl Launcher {
    pub fn new() -> Self {
        Self {
            finder: Self::index_apps_and_get_finder(),
            results: Vec::new(),
            list_state: Default::default(),
        }
    }

    pub fn reindex_apps(&mut self) {
        self.finder = Self::index_apps_and_get_finder()
    }

    fn index_apps_and_get_finder() -> finder::Finder {
        let indices = indexer::Indexer::default().index();
        finder::Finder::new(indices)
    }

    pub fn launch_focused_action(&self, command: &str) -> Option<()> {
        let exec = command
            /*
            * %f
            * A single file name, even if multiple files are selected.
            * The system reading the desktop entry should recognize that the program in question cannot handle multiple file arguments,
            * and it should should probably spawn and execute multiple copies of a program for each selected file
            * if the program is not able to handle additional file arguments.
            * If files are not on the local file system (i.e. are on HTTP or FTP locations),
            * the files will be copied to the local file system and %f will be expanded to point at the temporary file.
            * Used for programs that do not understand the URL syntax.
            */
            .replace("%f", "")

            /*
            * %F
            * A list of files. Use for apps that can open several local files at once.
            * Each file is passed as a separate argument to the executable program.
            */
            .replace("%F", "")

            /* A single URL. Local files may either be passed as file: URLs or as file path. */
            .replace("%u", "")

            /*
            * A list of URLs.
            * Each URL is passed as a separate argument to the executable program.
            * Local files may either be passed as file: URLs or as file path.
            */
            .replace("%U", "")

            /*
            * The Icon key of the desktop entry expanded as two arguments, first --icon and then the value of the Icon key.
            * Should not expand to any arguments if the Icon key is empty or missing.
            */
            .replace("%i", "")

            /* The translated name of the application as listed in the appropriate Name key in the desktop entry. */
            .replace("%c", "")

            /* The location of the desktop file as either a URI (if for example gotten from the vfolder system)
            * or a local filename or empty if no location is known.
            */
            .replace("%k", "");
        let deprecated_switches_regex = Regex::new(r"%(v|m|d|D|n|N)").unwrap();
        let exec = deprecated_switches_regex.replace_all(&exec, "");
        let spaces_regex = Regex::new(r"\s+").unwrap();
        let exec = spaces_regex.replace_all(&exec, " ");

        if let Ok(_c) = Command::new("/bin/sh")
            .arg("-c")
            .arg(&exec.to_string())
            .spawn()
        {
            Some(())
        } else {
            panic!("Unable to start app");
        }
    }
}

impl Plugin for Launcher {
    fn search(
        &mut self,
        query: &mut String,
        ctx: &egui::Context, // TODO remove context because you can access it from ui.ctx()
        ui: &mut Ui,
        gl_window: &GlutinWindowContext,
    ) -> PluginFlowControl {
        let plugin_flow_control = PluginFlowControl::Continue;

        if query.is_empty() {
            return plugin_flow_control;
        }

        self.results = self
            .finder
            .find(query)
            .into_iter()
            .map(|app_match| app_match.index)
            .cloned()
            .collect();

        self.list_state.mutate(ctx, self.results.len(), |idx| {
            self.results[idx].actions.len() + 1 // 1 primary 
        });

        ui.list(self.list_state, |mut ui| {
            if query.starts_with(",") {
                ui.row(|mut ui| {
                    if ui.primary_action("Refresh application cache").activated {
                        self.reindex_apps()
                    }
                });
                return;
            }

            for result in &self.results {
                ui.row(|mut ui| {
                    ui.label("Launch");

                    if ui.primary_action(&result.name).activated {
                        self.launch_focused_action(&result.exec);
                        query.clear();
                        gl_window.window().set_visible(false);
                    }

                    for action in &result.actions {
                        if ui.secondary_action(&action.name).activated {
                            self.launch_focused_action(&action.command);
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
        let disable_cursor = self.list_state.before_search(ctx);
        Preparation {
            disable_cursor,
            plugin_flow_control: PluginFlowControl::Continue,
        }
    }
}
