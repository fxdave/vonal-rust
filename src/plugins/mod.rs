use eframe::egui::{Context, Ui};

mod launcher;
mod math;

pub enum PluginFlowControl {
    /// check other plugins as well
    Continue,

    /// don't check other plugins
    Break,
}

pub trait Plugin {
    fn search(&mut self, query: &str, ui: &mut Ui) -> PluginFlowControl;
    fn before_search(&mut self, _ctx: &Context) {}
}

pub struct PluginManager {
    plugins: Vec<Box<dyn Plugin>>,
}

impl PluginManager {
    pub fn new() -> Self {
        Self {
            plugins: vec![
                Box::new(math::Math::new()),
                Box::new(launcher::Launcher::new()),
            ],
        }
    }

    pub fn search(&mut self, query: &str, ui: &mut Ui) {
        ui.horizontal(|ui| {
            ui.add_space(15.);
            ui.vertical(|ui| {
                for i in &mut self.plugins {
                    let flow_control = i.search(query, ui);
                    if let PluginFlowControl::Break = flow_control {
                        return;
                    }
                }
            });
        });
    }

    pub fn before_search(&mut self, ctx: &Context) {
        for plugin in &mut self.plugins {
            plugin.before_search(ctx);
        }
    }
}
