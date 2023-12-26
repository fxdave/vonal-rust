use super::*;
use egui::{Response, Widget, WidgetWithState};

pub struct List<T> {
    callback: Option<T>,
    limit: Option<i32>,
    initially_selected_row: i32,
}

impl<T: for<'a> FnOnce(&'a mut ListUi)> List<T> {
    pub fn new() -> Self {
        Self {
            callback: None,
            limit: None,
            initially_selected_row: 0,
        }
    }

    pub fn with_builder(mut self, cb: T) -> Self {
        self.callback = Some(cb);
        self
    }

    pub fn with_limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit as i32);
        self
    }

    pub fn with_initally_selected_row(mut self, initially_selected_row: i32) -> Self {
        self.initially_selected_row = initially_selected_row;
        self
    }
}

impl<T: for<'a> FnOnce(&'a mut ListUi)> Widget for List<T> {
    fn ui(self, ui: &mut egui::Ui) -> Response {
        let old_state = ListState::load_or_create(ui.ctx(), self.initially_selected_row);
        let mut new_state = old_state;
        let mut list_ui = ListUi::new(ui, &mut new_state, self.limit);
        (self.callback.unwrap())(&mut list_ui);
        let row_count = list_ui.count;
        let actual_row_length = list_ui.actual_row_length;
        new_state.update(ui.ctx(), row_count as usize, actual_row_length as usize);
        if new_state != old_state {
            ui.ctx().request_repaint();
            new_state.store(ui.ctx());
        }
        ui.label("")
    }
}

impl<T> WidgetWithState for List<T> {
    type State = ListState;
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test() {
        egui::__run_test_ui(|ui| {
            ui.add(List::new().with_builder(|list_ui| {
                list_ui.row(|row_ui| {
                    row_ui.primary_action("Hi");
                    row_ui.passive(|_ui| {});
                });
                list_ui.passive_row(|_ui| {});
            }));
        })
    }
}
