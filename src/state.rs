use std::process::Command;

use druid::{im, lens, Data, Lens, LensExt};
use regex::Regex;

#[derive(Clone, Data, Lens)]
pub struct AppAction {
    pub name: String,
    pub command: String,
}

#[derive(Clone, Lens, Data)]
pub struct AppEntry {
    pub actions: im::Vector<AppAction>,
}

#[derive(Clone, Lens, Data)]
pub struct FocusableResult {
    pub entry: AppEntry,
    pub focused: bool,
    pub focused_action: usize,
}

impl FocusableResult {
    pub fn select_right_action(&mut self) {
        let length = self.entry.actions.len().max(1) - 1;
        self.focused_action = (self.focused_action + 1).min(length);
    }
    pub fn select_left_action(&mut self) {
        self.focused_action = self.focused_action.max(1) - 1;
    }
    pub fn get_actions_with_focused_lens() -> impl Lens<Self, im::Vector<(AppAction, bool)>> {
        lens::Identity.map(
            // Expose shared data with children data
            |result: &Self| {
                result
                    .entry
                    .actions
                    .iter()
                    .cloned()
                    .enumerate()
                    .map(|(id, action)| (action, id == result.focused_action && result.focused))
                    .collect::<im::Vector<_>>()
            },
            |_result: &mut Self, _x: im::Vector<(AppAction, bool)>| {},
        )
    }
    pub fn launch_selected_action(&self) {
        let exec = self.entry.actions[self.focused_action].command

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
            std::process::exit(0);
        } else {
            panic!("Unable to start app");
        }
    }
}

#[derive(Clone, Data, Lens)]
pub struct VonalState {
    #[lens(name = "query_lens")]
    pub query: String,
    pub results: im::Vector<FocusableResult>,
}

impl VonalState {
    pub fn get_focused_id(&self) -> Option<usize> {
        self.results
            .iter()
            .enumerate()
            .find(|(_id, entry)| entry.focused)
            .map(|(id, _)| id)
    }

    pub fn get_focused_mut(&mut self) -> Option<&mut FocusableResult> {
        let id = self.get_focused_id()?;
        Some(&mut self.results[id])
    }

    pub fn get_focused(&self) -> Option<&FocusableResult> {
        let id = self.get_focused_id()?;
        Some(&self.results[id])
    }

    pub fn select_next_result(&mut self) {
        let old_focused = self.get_focused_id();
        if let Some(old_focused) = old_focused {
            let next_focused = old_focused + 1;
            if next_focused < self.results.len() {
                self.results[old_focused].focused = false;
                self.results[next_focused].focused = true;
            }
        } else if self.results.len() > 0 {
            self.results[0].focused = true;
        }
    }

    pub fn select_previous_result(&mut self) {
        let old_focused = self.get_focused_id();
        match old_focused {
            None | Some(0) => {}
            Some(old_focused) => {
                let prev_focused = old_focused - 1;
                if prev_focused < self.results.len() {
                    self.results[old_focused].focused = false;
                    self.results[prev_focused].focused = true;
                }
            }
        }
    }

    pub fn select_right_action(&mut self) {
        if let Some(old_focused) = self.get_focused_mut() {
            old_focused.select_right_action()
        }
    }

    pub fn select_left_action(&mut self) {
        if let Some(old_focused) = self.get_focused_mut() {
            old_focused.select_left_action()
        }
    }

    pub fn launch_selected(&self) {
        self.get_focused()
            .map(|focused| focused.launch_selected_action());
    }
}

impl VonalState {
    pub fn new() -> VonalState {
        VonalState {
            query: String::new(),
            results: im::vector![],
        }
    }
}
