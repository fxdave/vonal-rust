use egui::{Button, Color32, Context, Ui};

use crate::plugins::PluginContext;

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
            ..Default::default()
        }
    }

    pub fn before_search<'a>(&mut self, ctx: &mut PluginContext<'a>) {
        let disable_cursor = ctx.egui_ctx.input(|i| {
            i.key_pressed(egui::Key::ArrowUp)
                || i.key_pressed(egui::Key::ArrowDown)
                || (self.row != -1
                    && (i.key_pressed(egui::Key::ArrowLeft)
                        || i.key_pressed(egui::Key::ArrowRight)))
        });

        if disable_cursor {
            ctx.disable_cursor();
        }
    }

    /// # List state update.
    ///
    /// ## Dimension update:
    /// Because egui is not a declarative UI, it would require a repaint to get the dimension of the list.
    /// So we tell the dimensions to the list before rendering.
    ///
    /// ## focus / action update:
    /// It reads the context, searching for pressed keys, and it updates the UI according to the keys.
    pub fn update(
        &mut self,
        ctx: &Context,
        rows_length: usize,
        cols_length: impl Fn(usize) -> usize,
    ) {
        // prepare inputs
        let rows_length = rows_length as i32;

        // change row
        if ctx.input(|i| i.key_pressed(egui::Key::ArrowUp)) {
            self.row -= 1;
        }
        if ctx.input(|i| i.key_pressed(egui::Key::ArrowDown)) {
            self.row += 1;
        }

        // change col
        if ctx.input(|i| i.key_pressed(egui::Key::ArrowLeft)) {
            self.col -= 1;
        }
        if ctx.input(|i| i.key_pressed(egui::Key::ArrowRight)) {
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
        self.activate = ctx.input(|i| i.key_pressed(egui::Key::Enter))
    }
}

pub struct RowUi<'u> {
    ui: &'u mut Ui,
    list_state: ListState,
    col_i: i32,
    focused: bool,
    count_actions: bool,
    action_count: i32,
}

impl<'u> RowUi<'u> {
    pub fn new(ui: &'u mut Ui, list_state: ListState, focused: bool, count_actions: bool) -> Self {
        Self {
            col_i: 0,
            list_state,
            ui,
            focused,
            count_actions,
            action_count: 0,
        }
    }
    pub fn passive(&'u mut self, callback: impl FnOnce(&'u mut Ui)) {
        if self.count_actions {
            return;
        }
        callback(self.ui)
    }
    pub fn label(&mut self, name: &str) {
        if self.count_actions {
            return;
        }
        self.ui.colored_label(Color32::from_gray(200), name);
    }
    pub fn primary_action(&mut self, name: &str) -> RowUiAction {
        if self.count_actions {
            self.action_count += 1;
            return RowUiAction { activated: false };
        }
        let focused = self.focused && self.list_state.col == self.col_i;
        let bg = Color32::from_black_alpha(0);
        let fg = match focused {
            true => Color32::from_gray(255),
            false => Color32::from_gray(200),
        };
        self.col_i += 1;
        let activated = self
            .ui
            .scope(|ui| {
                ui.visuals_mut().override_text_color = Some(fg);
                let action_btn = Button::new(name).fill(bg);
                let activated =
                    ui.add(action_btn).clicked() || (focused && self.list_state.activate);
                return activated;
            })
            .inner;

        RowUiAction { activated }
    }
    pub fn secondary_action(&mut self, name: &str) -> RowUiAction {
        if self.count_actions {
            self.action_count += 1;
            return RowUiAction { activated: false };
        }
        let focused = self.focused && self.list_state.col == self.col_i;
        let bg = match focused {
            true => Color32::from_white_alpha(16),
            false => Color32::from_white_alpha(8),
        };
        let fg = match focused {
            true => Color32::from_gray(255),
            false => Color32::from_gray(200),
        };
        self.col_i += 1;

        let activated = self
            .ui
            .scope(|ui| {
                ui.visuals_mut().override_text_color = Some(fg);
                let action_btn = Button::new(name).fill(bg);
                let activated =
                    ui.add(action_btn).clicked() || (focused && self.list_state.activate);

                activated
            })
            .inner;
        RowUiAction { activated }
    }
}

pub struct RowUiAction {
    pub activated: bool,
}

pub struct ListUi {
    list_state: ListState,
    row_i: i32,
    length: Option<i32>,
    count_rows: bool,
    count: i32,
}

impl ListUi {
    fn new(list_state: ListState, length: Option<i32>, count_rows: bool) -> Self {
        Self {
            row_i: 0,
            list_state,
            length,
            count_rows,
            count: 0,
        }
    }
    pub fn row<'u>(&mut self, ui: &'u mut Ui, callback: impl FnOnce(RowUi)) {
        if self.count_rows {
            self.count += 1;
            return;
        }

        if let Some(max_len) = self.length {
            let current_row: i32 = self.list_state.row;
            let min: i32 = (current_row - max_len - 1).max(0);
            let max = min + max_len;
            let current_row_visible = self.row_i >= min && self.row_i < max;
            if !current_row_visible {
                self.row_i += 1;
                return;
            }
        }

        let focused = self.list_state.row == self.row_i;
        ui.horizontal(|ui| {
            let row_ui = RowUi::new(ui, self.list_state, focused, false);
            callback(row_ui);
        });
        self.row_i += 1;
    }

    pub fn passive<'u>(&mut self, callback: impl FnOnce()) {
        if !self.count_rows {
            callback();
        }
    }
}

pub trait CreateList {
    fn list<'u>(
        &'u mut self,
        ctx: &Context,
        list_state: &mut ListState,
        callback: impl FnMut(&mut ListUi, &mut Ui),
    );
    fn list_limited<'u>(
        &'u mut self,
        length: usize,
        list_state: ListState,
        callback: impl FnOnce(&mut ListUi, &mut Ui),
    );
}

impl CreateList for Ui {
    fn list<'u>(
        &'u mut self,
        ctx: &Context,
        list_state: &mut ListState,
        mut callback: impl FnMut(&mut ListUi, &mut Ui),
    ) {
        let mut listUi = ListUi::new(list_state, None, true);
        callback(&mut listUi, self);

        listUi.list_state.update(&ctx, listUi.count as usize, |r| 1);

        let mut listUi = ListUi::new(list_state, None, false);
        callback(&mut listUi, self);
    }
    fn list_limited<'u>(
        &'u mut self,
        length: usize,
        list_state: ListState,
        callback: impl FnOnce(&mut ListUi, &mut Ui),
    ) {
        callback(
            &mut ListUi::new(list_state, Some(length as i32), false),
            self,
        );
    }
}
