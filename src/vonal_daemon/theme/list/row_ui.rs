use egui::{Button, Color32, Ui};

pub struct RowUi<'a> {
    pub ui: &'a mut Ui,
    pub col: i32,
    pub focused: bool,
    pub focused_col: i32,
    pub enter_pressed: bool,
}

impl<'a> RowUi<'a> {
    #[allow(dead_code)]
    pub fn passive(&mut self, callback: impl FnOnce(&mut Ui)) {
        callback(self.ui)
    }
    pub fn label(&mut self, name: &str) {
        self.ui.colored_label(Color32::from_gray(200), name);
    }
    pub fn primary_action(&mut self, name: &str) -> RowUiAction {
        let focused = self.focused && self.focused_col == self.col;
        let bg = Color32::from_black_alpha(0);
        let fg = match focused {
            true => Color32::from_gray(255),
            false => Color32::from_gray(200),
        };
        self.col += 1;
        let activated = self
            .ui
            .scope(|ui| {
                ui.visuals_mut().override_text_color = Some(fg);
                let action_btn = Button::new(name).fill(bg);

                ui.add(action_btn).clicked() || (focused && self.enter_pressed)
            })
            .inner;

        RowUiAction { activated }
    }
    pub fn secondary_action(&mut self, name: &str) -> RowUiAction {
        let focused = self.focused && self.focused_col == self.col;
        let bg = match focused {
            true => Color32::from_white_alpha(16),
            false => Color32::from_white_alpha(8),
        };
        let fg = match focused {
            true => Color32::from_gray(255),
            false => Color32::from_gray(200),
        };
        self.col += 1;

        let activated = self
            .ui
            .scope(|ui| {
                ui.visuals_mut().override_text_color = Some(fg);
                let action_btn = Button::new(name).fill(bg);

                ui.add(action_btn).clicked() || (focused && self.enter_pressed)
            })
            .inner;
        RowUiAction { activated }
    }
}

pub struct RowUiAction {
    pub activated: bool,
}
