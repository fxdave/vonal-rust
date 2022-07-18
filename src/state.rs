use std::process::Command;

use druid::{im, lens, Data, Lens, LensExt};

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
        let length = self.entry.actions.len();
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
        if let Ok(_c) = Command::new("/bin/sh")
            .arg("-c")
            .arg(&self.entry.actions[self.focused_action].command)
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
