use eframe::{
    egui::{self, FontSelection, Id, Image, TextEdit, TextStyle},
    epaint::{vec2, Color32, FontId, Vec2},
};
use egui_extras::RetainedImage;
use plugins::{Plugin, PluginManager};

mod plugins;

fn main() {
    let options = eframe::NativeOptions {
        resizable: false,
        ..Default::default()
    };
    eframe::run_native("Vonal", options, Box::new(|_cc| Box::new(MyApp::default())));
}

struct MyApp {
    query: String,
    prompt_icon: RetainedImage,
    plugin_manager: PluginManager,
}

impl Default for MyApp {
    fn default() -> Self {
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

const SEARCH_INPUT_ID: &'static str = "#search_input";

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // TODO: move this to creation context
        let symbol_style: TextStyle = TextStyle::Name("asd".into());
        let mut style = (*ctx.style()).clone();
        style
            .text_styles
            .insert(symbol_style.clone(), FontId::proportional(42.));
        ctx.set_style(style);

        let frame = egui::containers::Frame {
            fill: Color32::from_rgb(16, 19, 22),
            ..Default::default()
        };

        if ctx.input().key_pressed(egui::Key::Escape) {
            if self.query.len() > 0 {
                self.query = String::new()
            } else {
                std::process::exit(0)
            }
        }
        self.plugin_manager.before_search(&ctx);
        let is_control = ctx.input().key_pressed(egui::Key::ArrowLeft)
            || ctx.input().key_pressed(egui::Key::ArrowRight);

        egui::CentralPanel::default().frame(frame).show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.add_sized(
                    [50., 50.],
                    Image::new(self.prompt_icon.texture_id(ctx), vec2(15., 15.)),
                );

                ui.add(
                    TextEdit::singleline(&mut self.query)
                        .interactive(!is_control)
                        .id(Id::new(SEARCH_INPUT_ID))
                        .frame(false)
                        .hint_text("Search something ...")
                        .font(FontSelection::FontId(FontId::proportional(20.)))
                        .margin(Vec2 { x: 0., y: 15. })
                        .desired_width(f32::INFINITY),
                );
                if let Some(mut state) = TextEdit::load_state(ui.ctx(), Id::new(SEARCH_INPUT_ID)) {
                    let ccursor = egui::text::CCursor::new(self.query.len());
                    state.set_ccursor_range(Some(egui::text::CCursorRange::one(ccursor)));
                    state.store(ui.ctx(), Id::new(SEARCH_INPUT_ID));
                }
                ui.memory().request_focus(Id::new(SEARCH_INPUT_ID));
            });

            self.plugin_manager.search(&self.query, ui)
        });
    }
}
