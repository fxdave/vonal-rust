use egui::{Context, Id};

#[derive(Default, Clone, Copy, PartialEq, Eq)]
pub struct ListState {
    /// nth result
    pub row: i32,
    /// nth action for multiple actions
    pub col: i32,
    /// activate key pressed
    pub activate: bool,
}

pub const LIST_ID: &str = "#main_list";

impl ListState {
    /// row: n means the nth row is selected
    /// row: -1 means no selection in the list,
    /// which is good because it lets you edit the search bar without restrictions
    pub fn new(selected_row: i32) -> Self {
        Self {
            row: selected_row,
            ..Default::default()
        }
    }

    pub fn get_disable_cursor(ctx: &Context) -> bool {
        let Some(state) = Self::load(ctx) else {
            return false;
        };

        ctx.input(|i| {
            let up = i.key_pressed(egui::Key::ArrowUp);
            let down = i.key_pressed(egui::Key::ArrowDown);
            let left = i.key_pressed(egui::Key::ArrowLeft);
            let right = i.key_pressed(egui::Key::ArrowRight);
            up || down || (state.row != -1 && (left || right))
        })
    }

    /// # List state update.
    ///
    /// ## Dimension update:
    /// Because egui is not a declarative UI, it would require a repaint to get the dimension of the list.
    /// So we tell the dimensions to the list before rendering.
    ///
    /// ## focus / action update:
    /// It reads the context, searching for pressed keys, and it updates the UI according to the keys.
    pub fn update(&mut self, ctx: &Context, rows_length: usize, actual_row_length: usize) {
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
            let min_idx = if actual_row_length as i32 == 0 { -1 } else { 0 };
            let max_idx = actual_row_length as i32 - 1;
            self.col = i32::min(self.col, max_idx);
            self.col = i32::max(self.col, min_idx);
        }

        // set activate key pressed state
        self.activate = ctx.input(|i| i.key_pressed(egui::Key::Enter))
    }

    pub fn load(ctx: &Context) -> Option<Self> {
        ctx.data_mut(|d| d.get_persisted(Id::new(LIST_ID)))
    }
    pub fn load_or_create(ctx: &Context, selected_row: i32) -> Self {
        ctx.data_mut(|d| d.get_persisted(Id::new(LIST_ID)))
            .unwrap_or(Self::new(selected_row))
    }
    pub fn store(self, ctx: &Context) {
        ctx.data_mut(|d| d.insert_persisted(Id::new(LIST_ID), self));
    }
    pub fn reset(ctx: &Context, selected_row: i32) {
        ctx.data_mut(|d| d.insert_persisted(Id::new(LIST_ID), Self::new(selected_row)));
    }
    pub fn clear(ctx: &Context) {
        let current: Option<Self> = ctx.data_mut(|d| d.get_persisted(Id::new(LIST_ID)));
        if current.is_some() {
            ctx.data_mut(|d| d.remove::<Self>(Id::new(LIST_ID)));
        }
    }
}
