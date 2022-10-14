use eframe::egui::{Context, Ui};

mod launcher;

pub trait Plugin {
    fn search(&mut self, query: &str, ui: &mut Ui);
    fn before_search(&mut self, _ctx: &Context) {}
}

pub struct PluginManager {
    plugins: Vec<Box<dyn Plugin>>,
}

impl PluginManager {
    pub fn new() -> Self {
        Self {
            plugins: vec![Box::new(launcher::LauncherPlugin::new())],
        }
    }
}

impl Plugin for PluginManager {
    fn search(&mut self, query: &str, ui: &mut Ui) {
        ui.horizontal(|ui| {
            ui.add_space(15.);
            ui.vertical(|ui| {
                for i in self.plugins.iter_mut() {
                    i.search(query, ui)
                }
            });
        });
    }
    fn before_search(&mut self, ctx: &Context) {
        for i in self.plugins.iter_mut() {
            i.before_search(ctx)
        }
    }
}
