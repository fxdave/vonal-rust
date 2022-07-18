mod plugins;
mod state;
use crate::state::{AppEntry, FocusableResult};
use druid::{
    theme::{BACKGROUND_LIGHT, LABEL_COLOR, TEXTBOX_BORDER_WIDTH, WINDOW_BACKGROUND_COLOR},
    widget::{
        Controller, CrossAxisAlignment, Flex, Label, List, MainAxisAlignment, Padding, Painter,
        Svg, SvgData, TextBox,
    },
    AppLauncher, Code, Color, Env, Event, EventCtx, Insets, LensExt, PlatformError, RenderContext,
    Widget, WidgetExt, WindowDesc,
};
use plugins::Plugin;
use state::{AppAction, VonalState};

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
                Code::ArrowDown => state.select_next_result(),
                Code::ArrowLeft => state.select_left_action(),
                Code::ArrowRight => state.select_right_action(),
                Code::ArrowUp => state.select_previous_result(),
                Code::Enter => state.launch_selected(),
                _ => {
                    state.results = self
                        .application_launcher_plugin
                        .search(&state.query)
                        .into_iter()
                        .map(|entry| FocusableResult {
                            entry: entry,
                            focused: false,
                            focused_action: 0,
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

fn build_row() -> impl Widget<FocusableResult> {
    let row_background_painter = Painter::new(|ctx, item: &FocusableResult, _| {
        let bounds = ctx.size().to_rect();
        if item.focused {
            ctx.fill(bounds, &Color::rgba(1., 1., 1., 0.26));
        }
    });

    let launch_text = Label::new("Launch").with_text_color(Color::rgba(1., 1., 1., 0.5));

    let actions = List::new(|| {
        Flex::column()
            .cross_axis_alignment(CrossAxisAlignment::Start)
            .with_child(Label::new(|item: &AppAction, _env: &_| item.name.clone()))
            .with_child(Label::new(|item: &AppAction, _env: &_| {
                item.command.clone()
            }))
    })
    .with_spacing(10.)
    .horizontal()
    .padding(Insets::new(10., 0., 0., 0.))
    .lens(FocusableResult::entry.then(AppEntry::actions))
    .env_scope(|env, app| {
        if app.focused {
            env.set(LABEL_COLOR, Color::rgba(1., 1., 1., 1.))
        } else {
            env.set(LABEL_COLOR, Color::rgba(1., 1., 1., 0.5))
        }
    });

    Flex::row()
        .with_flex_child(launch_text, 0.)
        .with_flex_child(actions, 1.)
        .main_axis_alignment(MainAxisAlignment::Center)
        .cross_axis_alignment(CrossAxisAlignment::Center)
        .padding(10.)
        .background(row_background_painter)
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
