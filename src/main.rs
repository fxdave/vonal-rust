use eframe::{
    egui::{self, FontSelection, Id, Image, TextEdit},
    epaint::{vec2, Color32, FontId, Vec2},
    CreationContext,
};
use egui_extras::RetainedImage;
use plugins::PluginManager;

mod plugins;

fn main() {
    let options = eframe::NativeOptions {
        resizable: false,
        ..Default::default()
    };
    eframe::run_native("Vonal", options, Box::new(|cc| Box::new(MyApp::new(cc))));
}

struct MyApp {
    query: String,
    prompt_icon: RetainedImage,
    plugin_manager: PluginManager,
}

impl MyApp {
    fn new(_cc: &CreationContext) -> Self {
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

const SEARCH_INPUT_ID: &str = "#search_input";

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // TODO: query top left corner and move window to the top
        // frame.set_window_pos(pos2(0., 0.));

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
        });
    }
}

impl MyApp {
    fn render_search_bar(&mut self, ui: &mut egui::Ui, _ctx: &egui::Context, disable_cursor: bool) {
        ui.add(
            TextEdit::singleline(&mut self.query)
                .interactive(!disable_cursor)
                .id(Id::new(SEARCH_INPUT_ID))
                .frame(false)
                .hint_text("Search something ...")
                .font(FontSelection::FontId(FontId::proportional(20.)))
                .margin(Vec2 { x: 0., y: 15. })
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
