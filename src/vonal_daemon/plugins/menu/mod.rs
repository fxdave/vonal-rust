use super::Plugin;

#[derive(Default)]
pub struct Menu {}

impl Menu {
    pub fn new() -> Menu {
        Menu::default()
    }
}

impl Plugin for Menu {
    fn search(&mut self, ui: &mut egui::Ui, ctx: &mut super::PluginContext<'_>) {}
}
