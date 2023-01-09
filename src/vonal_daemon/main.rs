#![feature(panic_info_message)]
use std::{
    fs, io::Read, os::unix::net::UnixListener, path::Path, process, sync::mpsc, time::Instant,
};
use std::{os::unix::net::UnixStream, thread};

use egui_glow::glow;
use raw_window_handle::{
    HasRawDisplayHandle, HasRawWindowHandle, RawDisplayHandle, RawWindowHandle,
};
use winit::dpi::PhysicalSize;
use winit::event::{Event, StartCause};
use winit::event_loop::{ControlFlow, EventLoop, EventLoopBuilder};
use winit::window::WindowBuilder;
use x11::xlib::_XDisplay;

mod app;
#[path = "../common.rs"]
mod common;
mod plugins;

fn main() {
    // Set less distracting panic message
    std::panic::set_hook(Box::new(|info| match info.message() {
        Some(message) => println!("Error: {}", message),
        None => println!("{}", info),
    }));

    let (tx, rx) = mpsc::channel();
    let socket_thread = thread::spawn(move || {
        start_socket(&tx);
    });
    start_gui(rx);
    socket_thread.join().expect("Couldn't join thread.");
}

fn start_socket(tx: &mpsc::Sender<UserEvent>) {
    let socket = Path::new(common::SOCKET_PATH);

    if UnixStream::connect(&socket).is_ok() {
        tx.send(UserEvent::Quit).unwrap();
        panic!("One daemon is already listening.")
    }

    // Delete old socket if necessary
    if socket.exists() {
        fs::remove_file(&socket).unwrap();
    }

    // Bind to socket
    let stream = if let Ok(stream) = UnixListener::bind(&socket) {
        stream
    } else {
        panic!("failed to bind socket")
    };

    println!("Server started, waiting for clients");

    // Iterate over clients, blocks if no client available
    for client in stream.incoming() {
        let mut buf = String::new();
        match client {
            Ok(mut stream) => {
                stream.read_to_string(&mut buf).unwrap();
                if tx.send(UserEvent::CliCommand(buf)).is_err() {
                    break;
                }
            }
            Err(_) => println!("error"),
        }
    }
}

fn start_gui(rx: mpsc::Receiver<UserEvent>) {
    let event_loop: EventLoop<UserEvent> = EventLoopBuilder::with_user_event().build();
    let (gl_window, gl) = create_display(&event_loop);
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
        Event::UserEvent(UserEvent::Quit) => process::exit(0),
        _ => {}
    });
}

fn hide_window(gl_window: &GlutinWindowContext) {
    gl_window.window().set_visible(false);
}

fn show_window(gl_window: &GlutinWindowContext, show: bool) {
    gl_window.window().set_visible(show);
    let monitor = get_focused_monitor(&gl_window).expect("pointer is not on the monitor");
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
pub struct GlutinWindowContext {
    pub window: winit::window::Window,
    pub gl_context: glutin::context::PossiblyCurrentContext,
    pub gl_display: glutin::display::Display,
    pub gl_surface: glutin::surface::Surface<glutin::surface::WindowSurface>,
}

impl GlutinWindowContext {
    // refactor this function to use `glutin-winit` crate eventually.
    // preferably add android support at the same time.
    #[allow(unsafe_code)]
    unsafe fn new(winit_window: winit::window::Window) -> Self {
        use glutin::prelude::*;
        use raw_window_handle::*;

        let raw_display_handle = winit_window.raw_display_handle();
        let raw_window_handle = winit_window.raw_window_handle();

        #[cfg(target_os = "linux")]
        let preference = glutin::display::DisplayApiPreference::EglThenGlx(Box::new(
            winit::platform::unix::register_xlib_error_hook,
        ));

        let gl_display = glutin::display::Display::new(raw_display_handle, preference).unwrap();

        let config_template = glutin::config::ConfigTemplateBuilder::new()
            .prefer_hardware_accelerated(None)
            .with_depth_size(0)
            .with_stencil_size(0)
            .with_transparency(false)
            .compatible_with_native_window(raw_window_handle)
            .build();

        let config = gl_display
            .find_configs(config_template)
            .unwrap()
            .next()
            .unwrap();

        let context_attributes =
            glutin::context::ContextAttributesBuilder::new().build(Some(raw_window_handle));
        // for surface creation.
        let (width, height): (u32, u32) = winit_window.inner_size().into();
        let surface_attributes =
            glutin::surface::SurfaceAttributesBuilder::<glutin::surface::WindowSurface>::new()
                .build(
                    raw_window_handle,
                    std::num::NonZeroU32::new(width).unwrap(),
                    std::num::NonZeroU32::new(height).unwrap(),
                );
        // start creating the gl objects
        let gl_context = gl_display
            .create_context(&config, &context_attributes)
            .unwrap();

        let gl_surface = gl_display
            .create_window_surface(&config, &surface_attributes)
            .unwrap();

        let gl_context = gl_context.make_current(&gl_surface).unwrap();

        gl_surface
            .set_swap_interval(
                &gl_context,
                glutin::surface::SwapInterval::Wait(std::num::NonZeroU32::new(1).unwrap()),
            )
            .unwrap();

        GlutinWindowContext {
            window: winit_window,
            gl_context,
            gl_display,
            gl_surface,
        }
    }

    fn window(&self) -> &winit::window::Window {
        &self.window
    }

    fn resize(&self, physical_size: winit::dpi::PhysicalSize<u32>) {
        use glutin::surface::GlSurface;
        self.gl_surface.resize(
            &self.gl_context,
            physical_size.width.try_into().unwrap(),
            physical_size.height.try_into().unwrap(),
        );
    }

    fn swap_buffers(&self) -> glutin::error::Result<()> {
        use glutin::surface::GlSurface;
        self.gl_surface.swap_buffers(&self.gl_context)
    }

    fn get_proc_address(&self, addr: &std::ffi::CStr) -> *const std::ffi::c_void {
        use glutin::display::GlDisplay;
        self.gl_display.get_proc_address(addr)
    }
}

fn get_focused_monitor(ctx: &GlutinWindowContext) -> Option<winit::monitor::MonitorHandle> {
    let raw_window_handle = ctx.window().raw_window_handle();
    let raw_display_handle = ctx.window().raw_display_handle();
    let pointer = query_pointer(raw_display_handle, raw_window_handle);
    let mut monitors = ctx.window.available_monitors().filter(|monitor| {
        let position = monitor.position();
        let size = monitor.size();

        let x_ok = position.x <= pointer.0 && pointer.0 < (position.x + size.width as i32);
        let y_ok = position.y <= pointer.1 && pointer.1 < (position.y + size.height as i32);

        return x_ok && y_ok;
    });

    monitors.next()
}

fn query_pointer(display: RawDisplayHandle, window: RawWindowHandle) -> (i32, i32) {
    let connection = if let RawDisplayHandle::Xlib(raw) = display {
        raw.display
    } else {
        panic!("(Connection) We only support X.org over Xlib")
    };
    let window = if let RawWindowHandle::Xlib(raw) = window {
        raw.window
    } else {
        panic!("(Window) We only support X.org over Xlib")
    };
    let mut r: x11::xlib::Window = Default::default();
    let mut c: x11::xlib::Window = Default::default();
    let mut x: i32 = Default::default();
    let mut y: i32 = Default::default();
    let mut rx: i32 = Default::default();
    let mut ry: i32 = Default::default();
    let mut m: u32 = Default::default();

    let ptr_r: *mut x11::xlib::Window = &mut r;
    let ptr_c: *mut x11::xlib::Window = &mut c;
    let ptr_x: *mut i32 = &mut x;
    let ptr_y: *mut i32 = &mut y;
    let ptr_rx: *mut i32 = &mut rx;
    let ptr_ry: *mut i32 = &mut ry;
    let ptr_m: *mut u32 = &mut m;
    unsafe {
        x11::xlib::XQueryPointer(
            connection as *mut _XDisplay,
            window,
            ptr_r,
            ptr_c,
            ptr_rx,
            ptr_ry,
            ptr_x,
            ptr_y,
            ptr_m,
        );
    }

    (rx, ry)
}

fn create_display(
    event_loop: &EventLoop<UserEvent>,
) -> (GlutinWindowContext, egui_glow::painter::Context) {
    let winit_window = WindowBuilder::new()
        .with_visible(false)
        .with_decorations(false)
        .with_resizable(false)
        .with_always_on_top(true)
        .with_inner_size(PhysicalSize {
            width: 10,
            height: 10,
        })
        .with_title("Vonal")
        .build(event_loop)
        .unwrap();

    let gl_window = unsafe { GlutinWindowContext::new(winit_window) };

    let gl = unsafe {
        glow::Context::from_loader_function(|s| {
            let s = std::ffi::CString::new(s)
                .expect("failed to construct C string from string for gl proc address");

            gl_window.get_proc_address(&s)
        })
    };

    (gl_window, gl)
}

#[derive(Debug)]
enum UserEvent {
    Quit,
    CliCommand(String),
}
