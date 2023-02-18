#![feature(panic_info_message)]
use std::error::Error;
use std::{fs, io::Read, os::unix::net::UnixListener, path::Path, sync::mpsc, time::Instant};
use std::{os::unix::net::UnixStream, thread};

use derive_more::{Display, Error};
use windowing::GlutinWindowContext;
use winit::event::{Event, StartCause};
use winit::event_loop::{ControlFlow, EventLoop, EventLoopBuilder};

mod app;
#[path = "../common.rs"]
mod common;
mod plugins;
mod windowing;

fn main() {
    let (tx, rx) = mpsc::channel();
    let socket_thread = thread::spawn(move || {
        if let Err(error) = start_socket(&tx) {
            tx.send(UserEvent::Quit).unwrap();
            eprintln!("Exiting because of this error: {:?}", error);
            return;
        }
    });
    start_gui(rx);
    socket_thread.join().expect("Couldn't join thread.");
}

fn start_socket(tx: &mpsc::Sender<UserEvent>) -> Result<(), Box<dyn Error>> {
    let socket = Path::new(common::SOCKET_PATH);

    if UnixStream::connect(&socket).is_ok() {
        return Err(Box::new(SocketError {
            message: "One daemon is already listening".into(),
        }));
    }

    // Delete old socket if necessary
    if socket.exists() {
        fs::remove_file(&socket)?;
    }

    // Bind to socket
    let stream = UnixListener::bind(&socket).or(Err(SocketError {
        message: "Failed to bind socket.".into(),
    }))?;

    println!("Server started, waiting for clients");

    // Iterate over clients, blocks if no client available
    for client in stream.incoming() {
        let mut stream = client?;
        let mut buf = String::new();
        stream.read_to_string(&mut buf).unwrap();
        tx.send(UserEvent::CliCommand(buf)).unwrap();
    }

    Ok(())
}

#[derive(Default, Debug, Display, Error)]
#[display(fmt = "{}", message)]
struct SocketError {
    message: String,
}

fn start_gui(rx: mpsc::Receiver<UserEvent>) {
    let event_loop: EventLoop<UserEvent> = EventLoopBuilder::with_user_event().build();
    let (gl_window, gl) = windowing::create_display(&event_loop);
    let gl = std::sync::Arc::new(gl);
    let mut egui_glow = egui_glow::EguiGlow::new(&event_loop, gl, None);
    let mut app = app::App::new();

    let proxy = event_loop.create_proxy();

    thread::spawn(move || {
        while let Ok(message) = rx.recv() {
            proxy.send_event(message).expect("Couldn't send message");
        }
    });

    event_loop.run(move |event, _, control_flow| match event {
        Event::RedrawRequested(_) => {
            *control_flow = redraw(&mut app, &mut egui_glow, &gl_window);
        }
        Event::WindowEvent { event, .. } => {
            use winit::event::WindowEvent;
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
        Event::UserEvent(UserEvent::CliCommand(command)) => match command.as_str() {
            "show" => show_window(&gl_window, true),
            "hide" => hide_window(&gl_window),
            "toggle" => {
                let show = !gl_window.window().is_visible().unwrap_or(false);
                show_window(&gl_window, show);
            }
            command => println!("Got command: {:?}", command),
        },
        Event::UserEvent(UserEvent::Quit) => control_flow.set_exit(),
        _ => {}
    });
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
    gl_window.window().set_inner_size(monitor.size());
}

fn redraw(
    app: &mut app::App,
    egui_glow: &mut egui_glow::EguiGlow,
    gl_window: &GlutinWindowContext,
) -> ControlFlow {
    let repaint_after = egui_glow.run(gl_window.window(), |egui_ctx| {
        #[allow(clippy::cast_possible_truncation)]
        egui_ctx.set_pixels_per_point(gl_window.window().scale_factor() as f32);
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

    // draw things behind egui here
    egui_glow.paint(gl_window.window());
    // draw things on top of egui here
    gl_window.swap_buffers().unwrap();

    control_flow
}

#[derive(Debug)]
enum UserEvent {
    Quit,
    CliCommand(String),
}
