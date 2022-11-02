use egui::{Context, Ui};

#[cfg(feature = "launcher_plugin")]
mod launcher;
#[cfg(feature = "math_plugin")]
mod math;

pub enum PluginFlowControl {
    /// check other plugins as well
    Continue,

    /// don't check other plugins
    Break,
}

pub trait Plugin {
    fn search(
        &mut self,
        query: &mut String,
        ui: &mut Ui,
        gl_window: &glutin::WindowedContext<glutin::PossiblyCurrent>,
    ) -> PluginFlowControl;
    fn before_search(
        &mut self,
        _query: &mut String,
        _ctx: &Context,
        _: &glutin::WindowedContext<glutin::PossiblyCurrent>,
    ) -> Preparation {
        Preparation {
            disable_cursor: false,
            plugin_flow_control: PluginFlowControl::Continue,
        }
    }
}

pub struct PluginManager {
    plugins: Vec<Box<dyn Plugin>>,
}

impl PluginManager {
    pub fn new() -> Self {
        Self {
            plugins: vec![
                #[cfg(feature = "math_plugin")]
                Box::new(math::Math::new()),
                #[cfg(feature = "launcher_plugin")]
                Box::new(launcher::Launcher::new()),
            ],
        }
    }

    pub fn search(
        &mut self,
        query: &mut String,
        ui: &mut Ui,
        gl_window: &glutin::WindowedContext<glutin::PossiblyCurrent>,
    ) {
        ui.horizontal(|ui| {
            ui.add_space(15.);
            ui.vertical(|ui| {
                for i in &mut self.plugins {
                    let flow_control = i.search(query, ui, gl_window);
                    if let PluginFlowControl::Break = flow_control {
                        return;
                    }
                }
            });
        });
    }

    pub fn before_search(
        &mut self,
        query: &mut String,
        ctx: &Context,
        gl_window: &glutin::WindowedContext<glutin::PossiblyCurrent>,
    ) -> Preparation {
        let mut disable_cursor = false;
        for plugin in &mut self.plugins {
            let preparation = plugin.before_search(query, ctx, gl_window);
            disable_cursor |= preparation.disable_cursor;
            if let PluginFlowControl::Break = preparation.plugin_flow_control {
                break;
            }
        }
        Preparation {
            disable_cursor,
            plugin_flow_control: PluginFlowControl::Break,
        }
    }
}

pub struct Preparation {
    /// if you move the cursor you have to hide the cursor as well
    /// otherwise the cursor will be jumping
    pub disable_cursor: bool,
    pub plugin_flow_control: PluginFlowControl,
}
