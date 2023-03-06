use egui::{Button, Color32, Context, Ui};

#[derive(Default, Clone, Copy)]
pub struct ListState {
    /// nth result
    row: i32,
    /// nth action for multiple actions
    col: i32,
    /// activate key pressed
    activate: bool,
}

impl ListState {
    /// row: n means the nth row is selected
    /// row: -1 means no selection in the list,
    /// which is good because it lets you edit the search bar without restrictions
    pub fn new(row: i32) -> Self {
        Self {
            row,
            col: 0,
            activate: false,
        }
    }

    pub fn before_search(&mut self, ctx: &Context) -> bool {
        let input = ctx.input();
        input.key_pressed(egui::Key::ArrowUp)
            || input.key_pressed(egui::Key::ArrowDown)
            || (self.row != -1
                && (input.key_pressed(egui::Key::ArrowLeft)
                    || input.key_pressed(egui::Key::ArrowRight)))
    }

    pub fn update(
        &mut self,
        ctx: &Context,
        rows_length: usize,
        cols_length: impl Fn(usize) -> usize,
    ) {
        // prepare inputs
        let rows_length = rows_length as i32;
        let input = ctx.input();

        // change row
        if input.key_pressed(egui::Key::ArrowUp) {
            self.row -= 1;
        }
        if input.key_pressed(egui::Key::ArrowDown) {
            self.row += 1;
        }

        // change col
        if input.key_pressed(egui::Key::ArrowLeft) {
            self.col -= 1;
        }
        if input.key_pressed(egui::Key::ArrowRight) {
            self.col += 1;
        }

        // restrict row
        self.row = i32::max(-1, i32::min(self.row, rows_length - 1));

        // restrict col
        if self.row == -1 {
            self.col = -1;
        } else {
            let min_idx = if cols_length(self.row as usize) as i32 == 0 {
                -1
            } else {
                0
            };
            let max_idx = cols_length(self.row as usize) as i32 - 1;
            self.col = i32::min(self.col, max_idx);
            self.col = i32::max(self.col, min_idx);
        }

        // set activate key pressed state
        self.activate = ctx.input().key_pressed(egui::Key::Enter)
    }
}

pub struct RowUi<'u> {
    ui: &'u mut Ui,
    list_state: ListState,
    col_i: i32,
    focused: bool,
}

impl<'u> RowUi<'u> {
    pub fn new(ui: &'u mut Ui, list_state: ListState, focused: bool) -> Self {
        Self {
            col_i: 0,
            list_state,
            ui,
            focused,
        }
    }
    pub fn label(&mut self, name: &str) {
        if self.focused {
            self.ui.colored_label(Color32::from_gray(255), name);
        } else {
            self.ui.colored_label(Color32::from_gray(200), name);
        }
    }
    pub fn primary_action(&mut self, name: &str) -> RowUiAction {
        let focused = self.focused && self.list_state.col == self.col_i;
        self.col_i += 1;

        let action_btn = Button::new(name).fill(Color32::from_black_alpha(0));
        let activated = self.ui.add(action_btn).clicked() || (focused && self.list_state.activate);
        RowUiAction { activated }
    }
    pub fn secondary_action(&mut self, name: &str) -> RowUiAction {
        let focused = self.focused && self.list_state.col == self.col_i;
        let color = match focused {
            true => Color32::from_white_alpha(16),
            false => Color32::from_white_alpha(8),
        };
        self.col_i += 1;

        let action_btn = Button::new(name).fill(color);
        let activated = self.ui.add(action_btn).clicked() || (focused && self.list_state.activate);
        RowUiAction { activated }
    }
}

pub struct RowUiAction {
    pub activated: bool,
}

pub struct ListUi<'u> {
    ui: &'u mut Ui,
    list_state: ListState,
    row_i: i32,
}

impl<'u> ListUi<'u> {
    pub fn new(ui: &'u mut Ui, list_state: ListState) -> Self {
        Self {
            row_i: 0,
            list_state,
            ui,
        }
    }

    pub fn row(&mut self, callback: impl FnOnce(RowUi)) {
        let focused = self.list_state.row == self.row_i;
        self.row_i += 1;
        self.ui.horizontal(|ui| {
            let row_ui = RowUi::new(ui, self.list_state, focused);
            callback(row_ui);
        });
    }
}

pub trait CreateList {
    fn list<'u>(&'u mut self, list_state: ListState, callback: impl FnOnce(ListUi<'u>));
}

impl CreateList for Ui {
    fn list<'u>(&'u mut self, list_state: ListState, callback: impl FnOnce(ListUi<'u>)) {
        callback(ListUi::new(self, list_state));
    }
}
