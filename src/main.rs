mod plugins;
mod state;
use druid::{
    im::{self, Vector},
    lens,
    widget::{
        Button, Controller, CrossAxisAlignment, Flex, Label, List, Padding, SizedBox, Svg, SvgData,
        TextBox,
    },
    AppLauncher, Code, Color, Env, Event, EventCtx, LensExt, PlatformError, UnitPoint, Widget,
    WidgetExt, WindowDesc,
};
use plugins::Plugin;
use state::VonalState;
use std::process::Command;

use crate::state::AppEntry;

struct DefaultFocusController;

impl<W: Widget<VonalState>> Controller<VonalState, W> for DefaultFocusController {
    fn event(
        &mut self,
        child: &mut W,
        ctx: &mut EventCtx,
        event: &Event,
        data: &mut VonalState,
        env: &Env,
    ) {
        if let Event::WindowConnected = event {
            ctx.request_focus()
        }

        child.event(ctx, event, data, env)
    }
}

struct SearchController {
    application_launcher_plugin: plugins::application_launcher::ApplicationLauncherPlugin,
}

impl<W: Widget<VonalState>> Controller<VonalState, W> for SearchController {
    fn event(
        &mut self,
        child: &mut W,
        ctx: &mut EventCtx,
        event: &Event,
        state: &mut VonalState,
        env: &Env,
    ) {
        println!("{:?}", ctx.has_focus());
        println!("{:?}", ctx.is_focused());

        if let Event::KeyUp(e) = event {
            match e.code {
                Code::ArrowDown => ctx.focus_next(),
                Code::ArrowUp => ctx.focus_prev(),
                _ => {
                    state.results = self.application_launcher_plugin.search(&state.query);
                }
            }
        }
        child.event(ctx, event, state, env)
    }
}

fn main() -> Result<(), PlatformError> {
    let state = VonalState::new();

    let window = WindowDesc::new(build_ui)
        .window_size((800., 400.))
        .resizable(false)
        .title("Vonal");

    AppLauncher::with_window(window).launch(state)?;

    Ok(())
}

fn build_row() -> SizedBox<(Vector<AppEntry>, AppEntry)> {
    Flex::row()
        .with_child(
            Label::new(
                |(_ctx, item): &(im::Vector<AppEntry>, AppEntry), _env: &_| item.name.clone(),
            )
            .align_vertical(UnitPoint::LEFT),
        )
        .with_flex_spacer(1.0)
        .with_child(
            Button::new("Open")
                .on_click(
                    |_ctx, (_shared, item): &mut (im::Vector<AppEntry>, AppEntry), _env| {
                        // We have access to both child's data and shared data.
                        // Remove element from right list.
                        // doing nothing
                        if let Ok(_c) = Command::new("/bin/sh")
                            .arg("-c")
                            .arg(&item.actions[0].command)
                            .spawn()
                        {
                            std::process::exit(0);
                        } else {
                            panic!("Unable to start app");
                        }
                    },
                )
                .fix_size(80.0, 20.0)
                .align_vertical(UnitPoint::CENTER),
        )
        .padding(10.0)
        .background(Color::rgb(0.5, 0.0, 0.5))
        .fix_height(50.0)
}

fn build_ui() -> impl Widget<VonalState> {
    let image_data = match include_str!("assets/chevron-right-solid.svg").parse::<SvgData>() {
        Ok(svg) => svg,
        Err(_err) => SvgData::default(),
    };
    let image = Svg::new(image_data)
        .fix_height(20.0)
        .fix_width(30.0)
        .padding(druid::Insets::new(0.0, 3.0, 10.0, 0.0));

    let search_box = TextBox::new()
        .with_placeholder("Try some keyword...")
        .with_text_size(22.0)
        .lens(VonalState::query_lens)
        .controller(DefaultFocusController)
        .controller(SearchController {
            application_launcher_plugin:
                plugins::application_launcher::ApplicationLauncherPlugin::load(),
        })
        .expand_width()
        .border(druid::Color::BLACK, 0.0);

    let results = List::new(|| build_row()).lens(lens::Identity.map(
        |state: &VonalState| (state.results.clone(), state.results.clone()),
        |_state: &mut VonalState, _x: (im::Vector<AppEntry>, im::Vector<AppEntry>)| {},
    ));

    Padding::new(
        10.0,
        Flex::column()
            .with_child(
                Flex::row()
                    .cross_axis_alignment(CrossAxisAlignment::Start)
                    .with_flex_child(image, 0.0)
                    .with_flex_child(search_box, 1.0),
            )
            .with_spacer(10.0)
            .with_child(results),
    )
}
