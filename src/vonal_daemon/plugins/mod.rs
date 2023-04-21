use egui::{Context, Ui};

use crate::{
    config::{ConfigBuilder, ConfigError},
    GlutinWindowContext,
};

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
    fn search(
        &mut self,
        query: &mut String,
        ui: &mut Ui,
        gl_window: &GlutinWindowContext,
    ) -> PluginFlowControl;
    fn before_search(
        &mut self,
        _query: &mut String,
        _ctx: &Context,
        _: &GlutinWindowContext,
    ) -> Preparation {
        Preparation {
            disable_cursor: false,
            plugin_flow_control: PluginFlowControl::Continue,
        }
    }
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
                #[cfg(feature = "math_plugin")]
                "math_plugin".to_string(),
                #[cfg(feature = "launcher_plugin")]
                "launcher_plugin".to_string(),
            ],
        )?;

        if self.config_plugins != plugins {
            self.plugins = Vec::new();

            for plugin in &plugins {
                match plugin.as_str() {
                    #[cfg(feature = "math_plugin")]
                    "math_plugin" => self.plugins.push(Box::new(math::Math::new())),
                    #[cfg(feature = "launcher_plugin")]
                    "launcher_plugin" => self.plugins.push(Box::new(launcher::Launcher::new())),
                    plugin_name => return Err(ConfigError::BadEntryError {
                        name: "plugins",
                        message: Some(format!(
                            "The specified plugin named \"{plugin_name}\" is unknown or vonal is not compiled with it."
                        )),
                    }),
                }
            }

            self.config_plugins = plugins;
        }

        for i in &mut self.plugins {
            builder = i.configure(builder)?
        }

        Ok(builder)
    }

    pub fn search(&mut self, query: &mut String, ui: &mut Ui, gl_window: &GlutinWindowContext) {
        ui.horizontal_top(|ui| {
            ui.add_space(15.);
            ui.vertical(|ui| {
                // don't search when there's nothing to search
                if query.is_empty() {
                    return;
                }

                for i in &mut self.plugins {
                    let flow_control = i.search(query, ui, gl_window);
                    if let PluginFlowControl::Break = flow_control {
                        return;
                    }
                }

                ui.add_space(10.);
            });
        });
    }

    pub fn before_search(
        &mut self,
        query: &mut String,
        ctx: &Context,
        gl_window: &GlutinWindowContext,
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
    /// if you move the focus by arrow keys, you have to hide the cursor.
    /// otherwise the cursor will be jumping
    pub disable_cursor: bool,
    pub plugin_flow_control: PluginFlowControl,
}
