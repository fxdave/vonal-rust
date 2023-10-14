use std::error::Error;

use egui::{Context, Ui};

use crate::{
    config::{ConfigBuilder, ConfigError},
    GlutinWindowContext,
};

//#[cfg(feature = "launcher_plugin")]
//mod launcher;
//#[cfg(feature = "math_plugin")]
//mod math;
#[cfg(feature = "pass_plugin")]
mod pass;

pub enum PluginFlowControl {
    /// check other plugins as well
    Continue,

    /// don't check other plugins
    Break,
}

pub struct PluginContext<'a> {
    pub query: &'a mut String,
    pub gl_window: &'a GlutinWindowContext,
    flow: PluginFlowControl,
    pub egui_ctx: &'a Context,
    disable_cursor: bool,
    error: Option<String>,
}

impl<'a> PluginContext<'a> {
    pub fn new(
        query: &'a mut String,
        gl_window: &'a GlutinWindowContext,
        egui_ctx: &'a Context,
    ) -> Self {
        Self {
            flow: PluginFlowControl::Continue,
            gl_window,
            query,
            egui_ctx,
            disable_cursor: false,
            error: Default::default(),
        }
    }
    pub fn break_flow(&mut self) {
        self.flow = PluginFlowControl::Break
    }
    pub fn disable_cursor(&mut self) {
        self.disable_cursor = true;
    }
    pub fn set_error(&mut self, message: String) {
        self.error = Some(message);
        self.break_flow();
    }
    pub fn verify_result<T>(&mut self, t: Result<T, Box<dyn Error>>) {
        if let Err(error) = t {
            self.set_error(error.to_string())
        }
    }
}

pub trait Plugin {
    /// Example:
    /// ```
    /// fn configure(&mut self, mut builder: ConfigBuilder) -> Result<ConfigBuilder, ConfigError> {
    ///     // primitive types
    ///     self.some_boolean = builder.get_or_create("some_boolean", false)?;
    ///     self.some_integer = builder.get_or_create("some_integer", 12)?;
    ///     self.some_float = builder.get_or_create("some_float", 0.3)?;
    ///     // objects
    ///     self.some_color = builder.get_or_create("some_color", Color32::from_rgb(12, 12, 12))?;
    ///     self.some_string = builder.get_or_create("some_string", String::from("something"))?;
    ///     self.some_array = builder.get_or_create("some_array", vec![1, 2, 3])?;
    ///     // table
    ///     let mut map = toml::map::Map::new();
    ///     map.insert(String::from("some_color_in_map"), Color32::from_rgb(12, 12, 12).to_config());
    ///     map.insert(String::from("some_number_in_map"), 42.to_config());
    ///     self.some_map = builder.get_or_create("some_map", map)?;
    ///     Ok(builder)
    /// }
    /// ```
    fn configure(&mut self, builder: ConfigBuilder) -> Result<ConfigBuilder, ConfigError> {
        Ok(builder)
    }
    fn search<'a>(&mut self, ui: &mut Ui, ctx: &mut PluginContext<'a>);
    fn before_search<'a>(&mut self, _ctx: &mut PluginContext<'a>) {}
}

#[derive(Default)]
pub struct PluginManager {
    plugins: Vec<Box<dyn Plugin>>,
    config_plugins: Vec<String>,
}
impl PluginManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn configure(&mut self, mut builder: ConfigBuilder) -> Result<ConfigBuilder, ConfigError> {
        let plugins = builder.get_or_create(
            "plugins",
            vec![
                //#[cfg(feature = "math_plugin")]
                //"math_plugin".to_string(),
                #[cfg(feature = "pass_plugin")]
                "pass_plugin".to_string(),
                //#[cfg(feature = "launcher_plugin")]
                //"launcher_plugin".to_string(),
            ],
        )?;

        if self.config_plugins != plugins {
            self.plugins = Vec::new();

            for plugin in &plugins {
                match plugin.as_str() {
                    //#[cfg(feature = "math_plugin")]
                    //"math_plugin" => self.plugins.push(Box::new(math::Math::new())),
                    #[cfg(feature = "pass_plugin")]
                    "pass_plugin" => self.plugins.push(Box::new(pass::Pass::new())),
                    //#[cfg(feature = "launcher_plugin")]
                    //"launcher_plugin" => self.plugins.push(Box::new(launcher::Launcher::new())),
                    //plugin_name => return Err(ConfigError::BadEntryError {
                    //    name: "plugins",
                    //    message: Some(format!(
                    //        "The specified plugin named \"{plugin_name}\" is unknown or vonal is not compiled with it."
                    //    )),
                    //}),
                    donothing => {}
                }
            }

            self.config_plugins = plugins;
        }

        for i in &mut self.plugins {
            builder = i.configure(builder)?
        }

        Ok(builder)
    }

    pub fn search<'a>(&mut self, ui: &mut Ui, ctx: &mut PluginContext<'a>) -> PostOperation {
        ui.horizontal_top(|ui| {
            ui.add_space(15.);
            ui.vertical(|ui| {
                // don't search when there's nothing to search
                if ctx.query.is_empty() {
                    return;
                }

                for i in &mut self.plugins {
                    i.search(ui, ctx);
                    if let PluginFlowControl::Break = ctx.flow {
                        break;
                    }
                }

                ui.add_space(10.);
            });
        });

        PostOperation {
            error: ctx.error.clone(),
        }
    }

    pub fn before_search<'a>(&mut self, ctx: &mut PluginContext<'a>) -> Preparation {
        for plugin in &mut self.plugins {
            plugin.before_search(ctx);
            if let PluginFlowControl::Break = ctx.flow {
                break;
            }
        }

        Preparation {
            disable_cursor: ctx.disable_cursor,
        }
    }
}

pub struct Preparation {
    /// if you move the focus by arrow keys, you have to hide the cursor.
    /// otherwise the cursor will be jumping
    pub disable_cursor: bool,
}
pub struct PostOperation {
    pub error: Option<String>,
}
