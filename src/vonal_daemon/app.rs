use egui::{
    text::CCursor,
    text_edit::{CCursorRange, TextEditOutput},
    vec2, Color32, FontId, FontSelection, Id, Image, RichText, TextEdit,
};
use egui_extras::RetainedImage;
use winit::dpi::PhysicalSize;

use crate::{
    config::{ConfigBuilder, ConfigError},
    plugins::PluginManager,
    GlutinWindowContext,
};

#[derive(Default)]
pub struct AppConfig {
    pub background: Color32,
    pub scale_factor: f32,
    pub show_mode_indicator: bool,
    pub placeholder: String,
}

pub struct App {
    pub config: AppConfig,
    pub query: String,
    pub reset_search_input_cursor: bool,
    prompt_icon: RetainedImage,
    plugin_manager: PluginManager,
    error: Option<String>,
}

impl App {
    pub fn new() -> Self {
        Self {
            config: AppConfig::default(),
            query: Default::default(),
            prompt_icon: egui_extras::RetainedImage::from_svg_bytes(
                "./assets/right.svg",
                include_bytes!("./assets/right.svg"),
            )
            .unwrap(),
            plugin_manager: PluginManager::new(),
            error: None,
            reset_search_input_cursor: false,
        }
    }
}

pub const SEARCH_INPUT_ID: &str = "#search_input";

impl App {
    #[allow(clippy::cast_possible_truncation)]
    #[allow(clippy::cast_sign_loss)]
    pub fn update(&mut self, ctx: &egui::Context, gl_window: &GlutinWindowContext) {
        // reset size
        gl_window.resize(PhysicalSize {
            width: 10,
            height: 10,
        });

        // Set wallpaper
        let frame = egui::containers::Frame {
            fill: match self.error {
                Some(_) => Color32::RED,
                None => self.config.background,
            },
            ..Default::default()
        };

        // Empty search bar / exit on escape
        self.handle_escape(ctx, gl_window);

        // Notify plugins before render
        let preparation = self
            .plugin_manager
            .before_search(&mut self.query, ctx, gl_window);

        // render window
        egui::CentralPanel::default().frame(frame).show(ctx, |ui| {
            if let Some(error) = self.error.as_ref() {
                self.render_error_screen(ui, error);
            } else {
                self.render_search_screen(ui, ctx, preparation, gl_window);
            }

            // reset window height
            if let Some(monitor) = gl_window.window().current_monitor() {
                let real_height = ui.cursor().min.y * ctx.pixels_per_point();
                let size = PhysicalSize {
                    width: monitor.size().width,
                    height: real_height as u32,
                };
                gl_window.resize(size);
                gl_window.window().set_inner_size(size);
                gl_window.window().set_max_inner_size(Some(size));
            }
        });
    }

    pub fn set_error(&mut self, error: Option<String>) {
        self.error = error;
    }

    pub fn configure(&mut self, mut builder: ConfigBuilder) -> Result<ConfigBuilder, ConfigError> {
        self.config.background =
            builder.get_or_create("background", Color32::from_rgb(16, 19, 22))?;
        self.config.scale_factor = builder.get_or_create("scale_factor", 1.0)?;
        self.config.show_mode_indicator = builder.get_or_create("show_mode_indicator", true)?;
        self.config.placeholder =
            builder.get_or_create("placeholder", "Search something ...".to_string())?;
        self.plugin_manager.configure(builder)
    }

    fn render_search_screen(
        &mut self,
        ui: &mut egui::Ui,
        ctx: &egui::Context,
        preparation: crate::plugins::Preparation,
        gl_window: &GlutinWindowContext,
    ) {
        ui.horizontal_top(|ui| {
            self.render_mode_indicator_icon(ui, ctx);
            self.render_search_bar(ui, ctx, preparation.disable_cursor);
        });

        // Let plugins render their results
        self.plugin_manager.search(&mut self.query, ui, gl_window);
    }

    fn render_error_screen(&self, ui: &mut egui::Ui, error: &String) {
        ui.add_space(15.);
        ui.horizontal(|ui| {
            ui.add_space(15.);
            ui.label(
                RichText::new(error)
                    .color(Color32::WHITE)
                    .font(FontId::proportional(20.)),
            );
        });
        ui.add_space(15.);
    }

    fn render_search_bar(&mut self, ui: &mut egui::Ui, _ctx: &egui::Context, disable_cursor: bool) {
        let TextEditOutput {
            mut state,
            response,
            ..
        } = TextEdit::singleline(&mut self.query)
            .interactive(!disable_cursor)
            .id(Id::new(SEARCH_INPUT_ID))
            .frame(false)
            .hint_text(&self.config.placeholder)
            .font(FontSelection::FontId(FontId::proportional(20.)))
            .margin(vec2(0., 15.))
            .desired_width(f32::INFINITY)
            .show(ui);

        if self.reset_search_input_cursor {
            // Create a new selection range
            let min = CCursor::new(self.query.len());
            let max = CCursor::new(self.query.len());
            let new_range = CCursorRange::two(min, max);
            state.set_ccursor_range(Some(new_range));
            state.store(ui.ctx(), response.id);
            self.reset_search_input_cursor = false;
        }

        ui.memory_mut(|memory| {
            memory.request_focus(Id::new(SEARCH_INPUT_ID));
        })
    }

    fn render_mode_indicator_icon(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        if !self.config.show_mode_indicator {
            ui.add_space(15.);
            return;
        }

        let size = self.prompt_icon.size_vec2();
        ui.add_sized(
            [50., 50.],
            Image::new(
                self.prompt_icon.texture_id(ctx),
                vec2(size.x * 15. / size.y, 15.),
            ),
        );
    }

    fn handle_escape(&mut self, ctx: &egui::Context, window: &GlutinWindowContext) {
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            if self.query.is_empty() {
                window.window().set_visible(false);
            }
            self.query = String::new();
        }
    }
}
