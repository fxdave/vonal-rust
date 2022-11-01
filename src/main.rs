use std::time::Instant;
use glutin::{
    dpi::PhysicalSize,
    event::{Event, StartCause},
};

mod app;
mod plugins;

fn main() {
    let event_loop = glutin::event_loop::EventLoopBuilder::with_user_event().build();
    let (gl_window, gl) = create_display(&event_loop);
    let gl = std::sync::Arc::new(gl);
    let mut egui_glow = egui_glow::EguiGlow::new(&event_loop, gl);
    let mut app = app::App::new();
    event_loop.run(move |event, _, control_flow| match event {
        Event::RedrawRequested(_) => {
            *control_flow = redraw(&mut app, &mut egui_glow, &gl_window);
        }
        Event::WindowEvent { event, .. } => {
            use glutin::event::WindowEvent;
            match event {
                WindowEvent::CloseRequested | WindowEvent::Destroyed => {
                    *control_flow = glutin::event_loop::ControlFlow::Exit;
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

            egui_glow.on_event(&event);
            gl_window.window().request_redraw();
        }
        Event::LoopDestroyed => {
            egui_glow.destroy();
        }
        Event::NewEvents(StartCause::ResumeTimeReached { .. }) => {
            gl_window.window().request_redraw();
        }
        _ => {}
    });
}

fn redraw(
    app: &mut app::App,
    egui_glow: &mut egui_glow::EguiGlow,
    gl_window: &glutin::ContextWrapper<glutin::PossiblyCurrent, glutin::window::Window>,
) -> glutin::event_loop::ControlFlow {
    let repaint_after = egui_glow.run(gl_window.window(), |egui_ctx| {
        #[allow(clippy::cast_possible_truncation)]
        egui_ctx.set_pixels_per_point(gl_window.window().scale_factor() as f32);
        app.update(egui_ctx, gl_window);
    });
    let control_flow = if repaint_after.is_zero() {
        gl_window.window().request_redraw();
        glutin::event_loop::ControlFlow::Poll
    } else if let Some(instant) = Instant::now().checked_add(repaint_after) {
        glutin::event_loop::ControlFlow::WaitUntil(instant)
    } else {
        glutin::event_loop::ControlFlow::Wait
    };

    // draw things behind egui here
    egui_glow.paint(gl_window.window());
    // draw things on top of egui here
    gl_window.swap_buffers().unwrap();

    control_flow
}

fn create_display(
    event_loop: &glutin::event_loop::EventLoop<()>,
) -> (
    glutin::WindowedContext<glutin::PossiblyCurrent>,
    egui_glow::glow::Context,
) {
    let window_builder = glutin::window::WindowBuilder::new()
        .with_decorations(false)
        .with_resizable(false)
        .with_always_on_top(true)
        .with_inner_size(PhysicalSize {
            width: 10,
            height: 10,
        })
        .with_title("Vonal");

    let gl_window = unsafe {
        glutin::ContextBuilder::new()
            .with_depth_buffer(0)
            .with_stencil_buffer(0)
            .with_vsync(true)
            .build_windowed(window_builder, event_loop)
            .unwrap()
            .make_current()
            .unwrap()
    };

    let gl = unsafe {
        egui_glow::glow::Context::from_loader_function(|s| gl_window.get_proc_address(s))
    };

    gl_window.window().set_always_on_top(true);
    if let Some(monitor) = gl_window.window().current_monitor() {
        let size = PhysicalSize {
            width: monitor.size().width,
            height: 50,
        };
        gl_window.resize(size);
        gl_window.window().set_inner_size(size);
        gl_window.window().set_min_inner_size(Some(size));
        gl_window.window().set_max_inner_size(Some(size));
        gl_window.window().set_outer_position(monitor.position());
    }

    (gl_window, gl)
}
