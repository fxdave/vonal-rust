mod plugins;
mod state;
use druid::{
    theme::{BACKGROUND_LIGHT, TEXTBOX_BORDER_WIDTH, WINDOW_BACKGROUND_COLOR},
    widget::{
        Controller, CrossAxisAlignment, Flex, Label, List, Padding, Painter, Svg, SvgData, TextBox,
    },
    AppLauncher, Code, Color, Env, Event, EventCtx, LensExt, PlatformError, RenderContext,
    UnitPoint, Widget, WidgetExt, WindowDesc,
};
use plugins::Plugin;
use state::{AppAction, VonalState};
use std::process::Command;

use crate::state::{AppEntry, Focusable};

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
        if let Event::KeyDown(e) = event {
            match e.code {
                Code::ArrowDown => {
                    let old_focused = state
                        .results
                        .iter()
                        .enumerate()
                        .find(|(_id, entry)| entry.focused)
                        .map(|(id, _)| id);
                    if let Some(old_focused) = old_focused {
                        let next_focused = old_focused + 1;
                        if next_focused < state.results.len() {
                            state.results[old_focused].focused = false;
                            state.results[next_focused].focused = true;
                        }
                    } else if state.results.len() > 0 {
                        state.results[0].focused = true;
                    }
                }
                Code::ArrowUp => {
                    let old_focused = state
                        .results
                        .iter()
                        .enumerate()
                        .find(|(_id, entry)| entry.focused)
                        .map(|(id, _)| id);
                    match old_focused {
                        None | Some(0) => {}
                        Some(old_focused) => {
                            let prev_focused = old_focused - 1;
                            if prev_focused < state.results.len() {
                                state.results[old_focused].focused = false;
                                state.results[prev_focused].focused = true;
                            }
                        }
                    }
                }
                Code::Enter => {
                    let focused = state
                        .results
                        .iter()
                        .enumerate()
                        .find(|(_id, entry)| entry.focused)
                        .map(|(id, _)| id);

                    if let Some(focused) = focused {
                        launch_app_entry(&state.results[focused].focusable)
                    }
                }
                _ => {
                    state.results = self
                        .application_launcher_plugin
                        .search(&state.query)
                        .into_iter()
                        .map(|entry| Focusable {
                            focusable: entry,
                            focused: false,
                        })
                        .collect();
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

    AppLauncher::with_window(window)
        .configure_env(|env, _| {
            env.set(TEXTBOX_BORDER_WIDTH, 0);
            env.set(WINDOW_BACKGROUND_COLOR, Color::BLACK);
            env.set(BACKGROUND_LIGHT, Color::BLACK);
        })
        .launch(state)?;

    Ok(())
}

fn build_row() -> impl Widget<Focusable<AppEntry>> {
    let row_background_painter = Painter::new(|ctx, item: &Focusable<AppEntry>, _| {
        let bounds = ctx.size().to_rect();
        if item.focused {
            ctx.fill(bounds, &Color::rgba(1., 1., 1., 0.26));
        }
    });

    Flex::row()
        .with_child(
            Label::new(|item: &Focusable<AppEntry>, _env: &_| item.focusable.name.clone())
                .align_vertical(UnitPoint::LEFT),
        )
        .with_child(
            List::new(|| Label::new(|item: &AppAction, _env: &_| item.name.clone()))
                .lens(Focusable::<AppEntry>::focusable.then(AppEntry::actions)),
        )
        .with_flex_spacer(1.0)
        .padding(10.0)
        .background(row_background_painter)
        .fix_height(50.0)
}

fn launch_app_entry(entry: &AppEntry) {
    if let Ok(_c) = Command::new("/bin/sh")
        .arg("-c")
        .arg(&entry.actions[0].command)
        .spawn()
    {
        std::process::exit(0);
    } else {
        panic!("Unable to start app");
    }
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
        .expand_width();

    let results = List::new(|| build_row()).lens(VonalState::results);

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
