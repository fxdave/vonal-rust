use std::error::Error;
use std::io::{BufRead, BufReader};
use std::sync::Arc;
use std::{fs, os::unix::net::UnixListener, path::Path, sync::mpsc, time::Instant};
use std::{os::unix::net::UnixStream, thread};

use crate::config::ConfigError;
use common::{Command, Commands};
use config::watcher::ConfigEvent;
use config::ConfigBuilder;
use derive_more::{Display, Error};
use egui_glow::glow::{self};
use egui_glow::painter::Context;
use windowing::GlutinWindowContext;
use winit::event::{Event, StartCause, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop, EventLoopBuilder};
use winit::platform::run_return::EventLoopExtRunReturn;

mod app;
#[path = "../common.rs"]
mod common;
mod config;
mod plugins;
mod theme;
mod utils;
mod windowing;

fn main() {
    let mut app = app::App::new();

    let (tx, rx) = mpsc::channel();
    let tx_clone = tx.clone();

    // start listening for vonalc commands
    thread::spawn(move || {
        if let Err(error) = start_socket(&tx_clone) {
            tx_clone.send(UserEvent::Quit).unwrap();
            eprintln!("Exiting because of this error: {error:?}");
        }
    });

    // start listening for config file changes
    let config_builder = app.configure(ConfigBuilder::new_safe()).unwrap();
    config_builder.save().unwrap();
    thread::spawn(move || {
        if let Err(error) = start_config_watcher(&tx) {
            tx.send(UserEvent::Quit).unwrap();
            eprintln!("Config watcher is closing: {error:?}");
        }
    });

    start_gui(app, rx);
}

fn start_socket(tx: &mpsc::Sender<UserEvent>) -> Result<(), Box<dyn Error>> {
    let socket = Path::new(common::SOCKET_PATH);

    if UnixStream::connect(socket).is_ok() {
        return Err(Box::new(SocketError {
            message: "One daemon is already listening".into(),
        }));
    }

    // Delete old socket if necessary
    if socket.exists() {
        fs::remove_file(socket)?;
    }

    // Bind to socket
    let stream = UnixListener::bind(socket).or(Err(SocketError {
        message: "Failed to bind socket.".into(),
    }))?;

    println!("Server started, waiting for clients");

    // Iterate over clients, blocks if no client available
    for client in stream.incoming() {
        let stream = client?;
        let mut reader = BufReader::new(stream);
        if let Err(error) = reader.fill_buf() {
            println!("{:?}", error);
        }
        let result = bincode::deserialize(reader.buffer());
        match result {
            Ok(commands) => tx.send(UserEvent::CliCommands(commands))?,
            Err(error) => {
                println!("{error:?}")
            }
        }
    }

    Ok(())
}

#[derive(Default, Debug, Display, Error)]
#[display(fmt = "{message}")]
struct SocketError {
    message: String,
}

fn start_gui(mut app: app::App, rx: mpsc::Receiver<UserEvent>) {
    let mut event_loop: EventLoop<UserEvent> = EventLoopBuilder::with_user_event().build();
    let (gl_window, gl) = windowing::create_display(&event_loop);
    let gl = std::sync::Arc::new(gl);
    let mut egui_glow = egui_glow::EguiGlow::new(&event_loop, gl.clone(), None);

    let proxy = event_loop.create_proxy();

    thread::spawn(move || {
        while let Ok(message) = rx.recv() {
            proxy.send_event(message).expect("Couldn't send message");
        }
    });

    event_loop.run_return(|event, _, control_flow| {
        handle_platform_event(
            event,
            control_flow,
            &mut app,
            &mut egui_glow,
            &gl_window,
            gl.clone(),
        )
    });
}

fn handle_platform_event(
    event: Event<UserEvent>,
    control_flow: &mut ControlFlow,
    app: &mut app::App,
    egui_glow: &mut egui_glow::EguiGlow,
    gl_window: &GlutinWindowContext,
    gl: Arc<Context>,
) {
    match event {
        Event::RedrawRequested(_) => {
            *control_flow = redraw(app, egui_glow, &gl_window, gl);
        }
        Event::WindowEvent { event, .. } => {
            match event {
                WindowEvent::CloseRequested | WindowEvent::Destroyed => {
                    gl_window.window().set_visible(false);
                }
                WindowEvent::Resized(ref physical_size) => {
                    gl_window.resize(*physical_size);
                }
                WindowEvent::ScaleFactorChanged {
                    ref new_inner_size, ..
                } => {
                    gl_window.resize(**new_inner_size);
                }
                _ => {}
            }

            if gl_window.window().is_visible().unwrap_or(false) {
                let event_response = egui_glow.on_event(&event);
                if event_response.repaint {
                    gl_window.window().request_redraw();
                }
            }
        }
        Event::LoopDestroyed => {
            egui_glow.destroy();
        }
        Event::NewEvents(StartCause::ResumeTimeReached { .. }) => {
            gl_window.window().request_redraw();
        }
        Event::UserEvent(UserEvent::CliCommands(commands)) => {
            parse_cli(commands.0, gl_window, app);
        }
        Event::UserEvent(UserEvent::Quit) => control_flow.set_exit(),
        Event::UserEvent(UserEvent::ConfigEvent(event)) => match event {
            ConfigEvent::Created => println!("Config file created"),
            ConfigEvent::Deleted => println!("Config file deleted"),
            ConfigEvent::Modified => {
                println!("Config file modified");
                let result = ConfigBuilder::new().and_then(|builder| app.configure(builder));
                match result {
                    Ok(_) => {
                        println!("Config has reloaded");
                        app.set_error(None);
                    }
                    Err(ConfigError::BadEntryError {
                        message: Some(message),
                        ..
                    }) => app.set_error(Some(message)),
                    Err(ConfigError::BadEntryError {
                        name,
                        message: None,
                    }) => app.set_error(Some(format!("Wrong config file entry at {name}."))),
                    Err(ConfigError::ParseError) => {
                        app.set_error(Some(format!("Config syntax error")))
                    }
                }
                gl_window.window().request_redraw();
            }
            ConfigEvent::None => (),
        },
        _ => {}
    }
}

fn parse_cli(commands: Vec<Command>, gl_window: &GlutinWindowContext, app: &mut app::App) {
    for command in &commands {
        match command {
            Command::Show => show_window(&gl_window, true),
            Command::Hide => hide_window(&gl_window),
            Command::Toggle => {
                let show = !gl_window.window().is_visible().unwrap_or(false);
                show_window(&gl_window, show);
            }
            Command::SetQuery { query } => {
                app.query = query.into();
                app.reset_search_input_cursor = true;
                gl_window.window().request_redraw();
            }
        }
    }
}

fn hide_window(gl_window: &GlutinWindowContext) {
    gl_window.window().set_visible(false);
}

fn show_window(gl_window: &GlutinWindowContext, show: bool) {
    gl_window.window().set_visible(show);
    let monitor = gl_window
        .get_focused_monitor()
        .expect("pointer is not on the monitor");
    gl_window.window().set_outer_position(monitor.position());
}

fn redraw(
    app: &mut app::App,
    egui_glow: &mut egui_glow::EguiGlow,
    gl_window: &GlutinWindowContext,
    gl: Arc<Context>,
) -> ControlFlow {
    let scale_factor = gl_window.window().scale_factor() as f32 * app.config.scale_factor;
    let repaint_after = egui_glow.run(gl_window.window(), |egui_ctx| {
        #[allow(clippy::cast_possible_truncation)]
        egui_ctx.set_pixels_per_point(scale_factor);
        app.update(egui_ctx, gl_window);
    });
    let control_flow = if repaint_after.is_zero() {
        gl_window.window().request_redraw();
        ControlFlow::Poll
    } else if let Some(instant) = Instant::now().checked_add(repaint_after) {
        ControlFlow::WaitUntil(instant)
    } else {
        ControlFlow::Wait
    };

    unsafe {
        use glow::HasContext as _;
        gl.clear_color(0.0, 0.0, 0.0, 0.0);
        gl.clear(glow::COLOR_BUFFER_BIT);
    }

    // draw things behind egui here
    egui_glow.paint(gl_window.window());
    // draw things on top of egui here
    gl_window.swap_buffers().unwrap();

    control_flow
}

fn start_config_watcher(tx: &mpsc::Sender<UserEvent>) -> Result<(), Box<dyn Error>> {
    let mut watcher = config::watcher::Watcher::new()?;

    for event in watcher.get_stream() {
        tx.send(UserEvent::ConfigEvent(event))?;
    }

    Ok(())
}

#[derive(Debug)]
enum UserEvent {
    Quit,
    CliCommands(Commands),
    ConfigEvent(ConfigEvent),
}
