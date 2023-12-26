use egui::Ui;

use super::{ListState, RowUi};

pub struct ListUi<'a> {
    pub ui: &'a mut Ui,
    pub list_state: &'a mut ListState,
    pub count: i32,
    pub length: Option<i32>,
    pub actual_row_length: i32,
}
impl<'a> ListUi<'a> {
    pub fn new(ui: &'a mut Ui, list_state: &'a mut ListState, length: Option<i32>) -> Self {
        Self {
            ui,
            count: 0,
            length,
            list_state,
            actual_row_length: 0,
        }
    }

    pub fn row(&mut self, cb: impl FnOnce(&mut RowUi)) {
        if let Some(max_len) = self.length {
            let current_row: i32 = self.list_state.row + 1;
            let min: i32 = (current_row - max_len).max(0);
            let max = min + max_len;
            let current_row_visible = self.count >= min && self.count < max;
            if !current_row_visible {
                self.count += 1;
                return;
            }
        }

        let focused = self.list_state.row == self.count;
        self.ui.horizontal(|ui| {
            let mut row_ui = RowUi {
                ui,
                col: 0,
                focused,
                focused_col: self.list_state.col,
                enter_pressed: self.list_state.activate,
            };
            cb(&mut row_ui);
            if focused {
                self.actual_row_length = row_ui.col;
            }
        });
        self.count += 1;
    }

    pub fn passive_row(&mut self, cb: impl FnOnce(&mut Ui)) {
        cb(self.ui)
    }
}
