use egui::{self, Button, Color32, Id, TextEdit, Ui};
use regex::Regex;
use std::process::Command;

use crate::app::SEARCH_INPUT_ID;

use self::indexer::traits::{AppIndex, IndexApps};

use super::{Plugin, PluginFlowControl, Preparation};

mod finder;
mod indexer;

pub struct Launcher {
    finder: finder::Finder,
    results: Vec<AppIndex>,
    focused_entry: usize,
    focused_entry_action: Option<usize>,
}

impl Launcher {
    pub fn new() -> Self {
        let indices = indexer::Indexer::default().index();
        let finder = finder::Finder::new(indices);
        Self {
            finder,
            results: Vec::new(),
            focused_entry: 0,
            focused_entry_action: None,
        }
    }

    pub fn launch_selected_action(&self) -> Option<()> {
        let exec = self.get_selected_command()?

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

    fn get_selected_command(&self) -> Option<&str> {
        let entry = self.get_focused_entry()?;
        Some(match self.focused_entry_action {
            Some(action) => &entry.actions.get(action)?.command,
            None => &entry.exec,
        })
    }

    fn get_focused_entry(&self) -> Option<&AppIndex> {
        self.results.get(self.focused_entry)
    }

    fn select_next(&mut self) {
        self.focused_entry = self.results.len().min(self.focused_entry + 1);
        self.focused_entry_action = None;
    }

    fn select_prev(&mut self) {
        self.focused_entry = self.focused_entry.max(1) - 1;
        self.focused_entry_action = None;
    }

    fn select_next_action(&mut self) {
        if let Some(entry) = self.get_focused_entry() {
            let len = entry.actions.len();
            self.focused_entry_action = match self.focused_entry_action {
                Some(action) if action >= len - 1 => Some(len - 1),
                Some(action) => Some(action + 1),
                None if len > 0 => Some(0),
                None => None,
            };
        }
    }

    fn select_prev_action(&mut self) {
        self.focused_entry_action = match self.focused_entry_action {
            Some(0) | None => None,
            Some(action) => Some(action - 1),
        }
    }
}

impl Plugin for Launcher {
    fn search(
        &mut self,
        query: &mut String,
        ui: &mut Ui,
        gl_window: &glutin::WindowedContext<glutin::PossiblyCurrent>,
    ) -> PluginFlowControl {
        if query.is_empty() {
            return PluginFlowControl::Continue;
        }

        self.results = self
            .finder
            .find(query)
            .into_iter()
            .map(|app_match| app_match.index)
            .cloned()
            .collect();

        // TODO: Cow will be faster then cloning always
        self.results
            .clone()
            .iter()
            .enumerate()
            .for_each(|(idx, result)| {
                ui.horizontal(|ui| {
                    if self.focused_entry == idx {
                        ui.colored_label(Color32::from_gray(255), "Launch");
                    } else {
                        ui.colored_label(Color32::from_gray(200), "Launch");
                    }

                    let default_action_btn =
                        Button::new(&result.name).fill(Color32::from_black_alpha(0));
                    if ui.add(default_action_btn).clicked() {
                        self.focused_entry = idx;
                        self.focused_entry_action = None;
                        self.launch_selected_action();
                        query.clear();
                        gl_window.window().set_visible(false);
                    }

                    for (action_idx, action) in result.actions.iter().enumerate() {
                        let color = match self.focused_entry_action {
                            Some(focused_ix) if focused_ix == action_idx => {
                                Color32::from_white_alpha(16)
                            }
                            _ => Color32::from_white_alpha(8),
                        };
                        let action_btn = Button::new(&action.name).fill(color);
                        if ui.add(action_btn).clicked() {
                            self.focused_entry = idx;
                            self.focused_entry_action = Some(action_idx);
                            self.launch_selected_action();
                            query.clear();
                            gl_window.window().set_visible(false);
                        }
                    }
                });
            });

        // reset cursor to the end, so we can use arrow keys for navigation in results instead of in input
        if let Some(mut state) = TextEdit::load_state(ui.ctx(), Id::new(SEARCH_INPUT_ID)) {
            let ccursor = egui::text::CCursor::new(query.len());
            state.set_ccursor_range(Some(egui::text::CCursorRange::one(ccursor)));
            state.store(ui.ctx(), Id::new(SEARCH_INPUT_ID));
        }

        PluginFlowControl::Continue
    }

    #[allow(clippy::useless_let_if_seq)]
    fn before_search(
        &mut self,
        query: &mut String,
        ctx: &egui::Context,
        gl_window: &glutin::WindowedContext<glutin::PossiblyCurrent>,
    ) -> Preparation {
        let mut disable_cursor = false;
        if ctx.input().key_pressed(egui::Key::ArrowDown) {
            self.select_next();
            disable_cursor = true;
        }
        if ctx.input().key_pressed(egui::Key::ArrowUp) {
            self.select_prev();
            disable_cursor = true;
        }
        if ctx.input().key_pressed(egui::Key::ArrowLeft) {
            self.select_prev_action();
            disable_cursor = true;
        }
        if ctx.input().key_pressed(egui::Key::ArrowRight) {
            self.select_next_action();
            disable_cursor = true;
        }
        if ctx.input().key_pressed(egui::Key::Enter) {
            self.launch_selected_action();
            query.clear();
            gl_window.window().set_visible(false);
            disable_cursor = true;
        }

        Preparation {
            disable_cursor,
            plugin_flow_control: PluginFlowControl::Continue,
        }
    }
}
