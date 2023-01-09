use egui::{self, Button, Color32, Id, TextEdit, Ui};
use regex::Regex;
use std::process::Command;

use crate::{app::SEARCH_INPUT_ID, GlutinWindowContext};

use self::indexer::traits::{AppIndex, IndexApps};

use super::{Plugin, PluginFlowControl, Preparation};

mod finder;
mod indexer;

#[derive(Default, PartialEq, Eq)]
struct Counter(Option<usize>);

impl Counter {
    fn get(&self) -> Option<usize> {
        self.0
    }

    fn next(&mut self, len: usize) {
        self.0 = match self.0 {
            _ if len == 0 => None,
            None => Some(0),
            Some(x) if x + 1 == len => Some(x),
            Some(x) => Some(x + 1),
        }
    }

    fn prev(&mut self, len: usize) {
        self.0 = match self.0 {
            _ if len == 0 => None,
            None | Some(0) => None,
            Some(x) => Some(x - 1),
        }
    }

    fn reset(&mut self) {
        self.0 = None
    }
}

#[derive(Default)]
pub struct Focus {
    /// The main command
    entry: Counter,
    /// Only the sub commands
    entry_action: Counter,
}

impl Focus {
    fn entry(&self) -> Option<usize> {
        return self.entry.get();
    }

    fn entry_action(&self) -> Option<usize> {
        return self.entry_action.get();
    }

    fn set(&mut self, entry: Option<usize>, action: Option<usize>) {
        self.entry = Counter(entry);
        self.entry_action = Counter(action);
    }

    fn is_focused(&self, entry: usize, action: Option<usize>) -> bool {
        return self.is_entry_focused(entry) && self.is_action_focused(action);
    }

    fn is_entry_focused(&self, entry: usize) -> bool {
        return self.entry == Counter(Some(entry));
    }

    fn is_action_focused(&self, action: Option<usize>) -> bool {
        return self.entry_action == Counter(action);
    }

    fn focus_next(&mut self, entries_len: usize) {
        self.entry.next(entries_len);
        self.entry_action.reset();
    }

    fn focus_prev(&mut self, entries_len: usize) {
        self.entry.prev(entries_len);
        self.entry_action.reset();
    }

    fn focus_next_action(&mut self, actions_len: usize) {
        self.entry_action.next(actions_len);
    }

    fn focus_prev_action(&mut self, actions_len: usize) {
        self.entry_action.prev(actions_len);
    }

    fn set_default(&mut self, entries_len: usize) {
        self.entry = Counter(match self.entry.get() {
            None if entries_len > 0 => Some(0),
            x => x,
        })
    }
}

pub struct Launcher {
    finder: finder::Finder,
    results: Vec<AppIndex>,
    focus: Focus,
}

impl Launcher {
    pub fn new() -> Self {
        Self {
            finder: Self::index_apps_and_get_finder(),
            results: Vec::new(),
            focus: Focus::default(),
        }
    }

    pub fn reindex_apps(&mut self) {
        self.finder = Self::index_apps_and_get_finder()
    }

    fn index_apps_and_get_finder() -> finder::Finder {
        let indices = indexer::Indexer::default().index();
        finder::Finder::new(indices)
    }

    pub fn launch_focused_action(&self) -> Option<()> {
        let exec = self.get_focused_command()?

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

    fn get_focused_command(&self) -> Option<&str> {
        let entry = self.get_focused_entry()?;
        Some(match self.focus.entry_action() {
            Some(action) => &entry.actions.get(action)?.command,
            None => &entry.exec,
        })
    }

    fn get_focused_entry(&self) -> Option<&AppIndex> {
        self.focus.entry().and_then(|i| self.results.get(i))
    }

    fn launch_action(&mut self, idx: usize, action: Option<usize>) {
        self.focus.set(Some(idx), action);
        self.launch_focused_action();
    }
}

impl Plugin for Launcher {
    fn search(
        &mut self,
        query: &mut String,
        ctx: &egui::Context,
        ui: &mut Ui,
        gl_window: &GlutinWindowContext,
    ) -> PluginFlowControl {
        let enter_pressed = ctx.input().key_pressed(egui::Key::Enter);

        if query.is_empty() {
            return PluginFlowControl::Continue;
        }

        if query.starts_with(",") {
            let refresh_btn = ui
                .add(Button::new("Refresh application cache").fill(Color32::from_white_alpha(16)));
            if refresh_btn.clicked() || enter_pressed {
                self.reindex_apps()
            }
            return PluginFlowControl::Continue;
        }

        self.results = self
            .finder
            .find(query)
            .into_iter()
            .map(|app_match| app_match.index)
            .cloned()
            .collect();

        self.focus.set_default(self.results.len());

        // TODO: Cow will be faster then cloning always
        self.results
            .clone()
            .iter()
            .enumerate()
            .for_each(|(idx, result)| {
                ui.horizontal(|ui| {
                    if self.focus.is_entry_focused(idx) {
                        ui.colored_label(Color32::from_gray(255), "Launch");
                    } else {
                        ui.colored_label(Color32::from_gray(200), "Launch");
                    }

                    let default_action_btn =
                        ui.add(Button::new(&result.name).fill(Color32::from_black_alpha(0)));
                    let focused = self.focus.is_focused(idx, None);
                    if default_action_btn.clicked() || (focused && enter_pressed) {
                        self.launch_action(idx, None);
                        query.clear();
                        gl_window.window().set_visible(false);
                    }

                    for (action_idx, action) in result.actions.iter().enumerate() {
                        let focused = self.focus.is_focused(idx, Some(action_idx));
                        let color = match focused {
                            true => Color32::from_white_alpha(16),
                            false => Color32::from_white_alpha(8),
                        };
                        let action_btn = Button::new(&action.name).fill(color);
                        if ui.add(action_btn).clicked() || (focused && enter_pressed) {
                            self.launch_action(idx, Some(action_idx));
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
        _query: &mut String,
        ctx: &egui::Context,
        _gl_window: &GlutinWindowContext,
    ) -> Preparation {
        let mut disable_cursor = false;
        if ctx.input().key_pressed(egui::Key::ArrowDown) {
            self.focus.focus_next(self.results.len());
            disable_cursor = true;
        }
        if ctx.input().key_pressed(egui::Key::ArrowUp) {
            self.focus.focus_prev(self.results.len());
            disable_cursor = true;
        }
        if ctx.input().key_pressed(egui::Key::ArrowLeft) {
            if let Some(entry) = self.focus.entry().and_then(|i| self.results.get(i)) {
                self.focus.focus_prev_action(entry.actions.len());
            }
            disable_cursor = true;
        }
        if ctx.input().key_pressed(egui::Key::ArrowRight) {
            if let Some(entry) = self.focus.entry().and_then(|i| self.results.get(i)) {
                self.focus.focus_next_action(entry.actions.len());
            }
            disable_cursor = true;
        }

        Preparation {
            disable_cursor,
            plugin_flow_control: PluginFlowControl::Continue,
        }
    }
}
