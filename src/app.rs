use egui::{vec2, Color32, FontId, FontSelection, Id, Image, TextEdit};
use egui_extras::RetainedImage;
use glutin::dpi::PhysicalSize;

use crate::plugins::PluginManager;

pub struct App {
    query: String,
    prompt_icon: RetainedImage,
    plugin_manager: PluginManager,
}

impl App {
    pub fn new() -> Self {
        Self {
            query: String::new(),
            prompt_icon: egui_extras::RetainedImage::from_svg_bytes(
                "./assets/right.svg",
                include_bytes!("./assets/right.svg"),
            )
            .unwrap(),
            plugin_manager: PluginManager::new(),
        }
    }
}

pub const SEARCH_INPUT_ID: &str = "#search_input";

impl App {

    #[allow(clippy::cast_possible_truncation)]
    #[allow(clippy::cast_sign_loss)]
    pub fn update(
        &mut self,
        ctx: &egui::Context,
        gl_window: &glutin::WindowedContext<glutin::PossiblyCurrent>,
    ) {
        // reset size
        gl_window.resize(PhysicalSize {
            width: 10,
            height: 10,
        });

        // Set wallpaper
        let frame = egui::containers::Frame {
            fill: Color32::from_rgb(16, 19, 22),
            ..Default::default()
        };

        // Empty search bar / exit on escape
        self.handle_escape(ctx);

        // Notify plugins before render
        let preparation = self.plugin_manager.before_search(&self.query, ctx);

        // render window
        egui::CentralPanel::default().frame(frame).show(ctx, |ui| {
            ui.horizontal(|ui| {
                self.render_mode_indicator_icon(ui, ctx);
                self.render_search_bar(ui, ctx, preparation.disable_cursor);
            });

            // Let plugins render their results
            self.plugin_manager.search(&self.query, ui);

            // reset window height
            if let Some(monitor) = gl_window.window().current_monitor() {
                let real_height = ui.cursor().min.y * ctx.pixels_per_point();
                let size = PhysicalSize {
                    width: monitor.size().width,
                    height: real_height as u32,
                };
                gl_window.resize(size);
                gl_window.window().set_inner_size(size);
                //gl_window.window().set_min_inner_size(Some(size));
                gl_window.window().set_max_inner_size(Some(size));
            }
        });
    }

    fn render_search_bar(&mut self, ui: &mut egui::Ui, _ctx: &egui::Context, disable_cursor: bool) {
        ui.add(
            TextEdit::singleline(&mut self.query)
                .interactive(!disable_cursor)
                .id(Id::new(SEARCH_INPUT_ID))
                .frame(false)
                .hint_text("Search something ...")
                .font(FontSelection::FontId(FontId::proportional(20.)))
                .margin(vec2(0., 15.))
                .desired_width(f32::INFINITY),
        );
        ui.memory().request_focus(Id::new(SEARCH_INPUT_ID));
    }

    fn render_mode_indicator_icon(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        ui.add_sized(
            [50., 50.],
            Image::new(self.prompt_icon.texture_id(ctx), vec2(15., 15.)),
        );
    }

    fn handle_escape(&mut self, ctx: &egui::Context) {
        if ctx.input().key_pressed(egui::Key::Escape) {
            if self.query.is_empty() {
                std::process::exit(0)
            }
            self.query = String::new();
        }
    }
}
