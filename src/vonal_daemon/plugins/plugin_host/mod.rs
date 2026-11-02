use libloading::{Library, Symbol};
use serde::Deserialize;

use super::Plugin;
use crate::config::{ConfigBuilder, ConfigError};
use std::{
    error::Error,
    ffi::{c_char, c_void, CString},
};

struct LowLevelExtension {
    lib: Library,
    instance: *mut c_void,
}

impl LowLevelExtension {
    fn new(path: &str) -> Result<Self, Box<dyn Error>> {
        unsafe {
            let lib = libloading::Library::new(path)?;
            let func_new: libloading::Symbol<unsafe extern "C" fn() -> *mut c_void> =
                lib.get(b"_new")?;

            Ok(Self {
                instance: func_new(),
                lib,
            })
        }
    }

    fn configure(&mut self, current_config_json: String) -> Result<String, Box<dyn Error>> {
        let func_configure: Symbol<
            unsafe extern "C" fn(*mut c_void, *const c_char) -> *mut c_char,
        > = unsafe { self.lib.get(b"_configure") }?;
        let input = CString::new(current_config_json)?;
        let output = unsafe { (func_configure)(self.instance, input.as_ptr()) };
        let output = unsafe { CString::from_raw(output) };
        let output = output.to_string_lossy().to_string();
        Ok(output)
    }

    fn search(&mut self, context: String) -> Vec<Action> {
        vec![Action::Continue]
    }
}

impl Drop for LowLevelExtension {
    fn drop(&mut self) {
        unsafe {
            let func_drop: Symbol<unsafe extern "C" fn(*mut c_void) -> ()> =
                unsafe { self.lib.get(b"_drop").unwrap() };
            (func_drop)(self.instance);
        }
    }
}

struct Extension {
    ext: LowLevelExtension,
}

impl Extension {
    fn call_configure(
        &mut self,
        mut builder: ConfigBuilder,
    ) -> Result<ConfigBuilder, Box<dyn std::error::Error>> {
        let current = serde_json::to_string(&builder.config)?;
        let new = self.ext.configure(current)?;
        builder.config = toml::from_str(&new)?;
        Ok(builder)
    }
}

impl Plugin for Extension {
    fn configure(&mut self, mut builder: ConfigBuilder) -> Result<ConfigBuilder, ConfigError> {
        Ok(self.call_configure(builder)?)
    }

    fn search(&mut self, ui: &mut egui::Ui, ctx: &mut super::PluginContext<'_>) {
        // TODO: call a plugin with these
    }
    fn before_search(&mut self, ctx: &mut super::PluginContext<'_>) {}
}

#[derive(Default)]
pub struct PluginHost {
    extensions: Vec<Extension>,
}

#[derive(Deserialize)]
enum Action {
    CreateRow { id: u64 },
    Continue,
    Break,
    SetQuery(String),
    HideWindow,
    ShowWindow,
    ToggleWindow,
}

impl PluginHost {
    fn new() -> PluginHost {
        PluginHost::default()
    }

    fn load_extensions(&mut self) {
        // load from dir
    }

    fn configure_extensions(
        &mut self,
        mut builder: ConfigBuilder,
    ) -> Result<ConfigBuilder, ConfigError> {
        let mut current = builder;

        for ext in &mut self.extensions {
            current = ext.configure(current)?;
        }

        Ok(current)
    }
}

impl Plugin for PluginHost {
    fn configure(&mut self, mut builder: ConfigBuilder) -> Result<ConfigBuilder, ConfigError> {
        self.load_extensions();
        let builder = self.configure_extensions(builder)?;
        Ok(builder)
    }

    fn search(&mut self, ui: &mut egui::Ui, ctx: &mut super::PluginContext<'_>) {
        // TODO: call a plugin with these
    }
    fn before_search(&mut self, ctx: &mut super::PluginContext<'_>) {}
}
