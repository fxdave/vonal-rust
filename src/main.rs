use glutin::dpi::PhysicalSize;

mod app;
mod plugins;

fn main() {
    let event_loop = glutin::event_loop::EventLoopBuilder::with_user_event().build();
    let (gl_window, gl) = create_display(&event_loop);
    let gl = std::sync::Arc::new(gl);

    let mut egui_glow = egui_glow::EguiGlow::new(&event_loop, gl.clone());

    let mut app = app::App::new();
    event_loop.run(move |event, _, control_flow| {
        let mut redraw = || {
            let quit = false;
            let repaint_after = egui_glow.run(gl_window.window(), |egui_ctx| {
                #[allow(clippy::cast_possible_truncation)]
                egui_ctx.set_pixels_per_point(gl_window.window().scale_factor() as f32);
                app.update(egui_ctx, &gl_window);
            });

            *control_flow = if quit {
                glutin::event_loop::ControlFlow::Exit
            } else if repaint_after.is_zero() {
                gl_window.window().request_redraw();
                glutin::event_loop::ControlFlow::Poll
            } else if let Some(repaint_after_instant) =
                std::time::Instant::now().checked_add(repaint_after)
            {
                glutin::event_loop::ControlFlow::WaitUntil(repaint_after_instant)
            } else {
                glutin::event_loop::ControlFlow::Wait
            };

            {
                unsafe {
                    use egui_glow::glow::HasContext as _;
                    gl.clear(egui_glow::glow::COLOR_BUFFER_BIT);
                }

                // draw things behind egui here
                egui_glow.paint(gl_window.window());

                // draw things on top of egui here
                gl_window.swap_buffers().unwrap();
            }
        };

        match event {
            glutin::event::Event::RedrawRequested(_) if !cfg!(windows) => redraw(),

            glutin::event::Event::WindowEvent { event, .. } => {
                use glutin::event::WindowEvent;
                if matches!(event, WindowEvent::CloseRequested | WindowEvent::Destroyed) {
                    *control_flow = glutin::event_loop::ControlFlow::Exit;
                }

                if let glutin::event::WindowEvent::Resized(physical_size) = &event {
                    gl_window.resize(*physical_size);
                } else if let glutin::event::WindowEvent::ScaleFactorChanged {
                    new_inner_size,
                    ..
                } = &event
                {
                    gl_window.resize(**new_inner_size);
                }
                egui_glow.on_event(&event);
                gl_window.window().request_redraw();
            }
            glutin::event::Event::LoopDestroyed => {
                egui_glow.destroy();
            }
            glutin::event::Event::NewEvents(glutin::event::StartCause::ResumeTimeReached {
                ..
            }) => {
                gl_window.window().request_redraw();
            }

            _ => (),
        }
    });
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
        gl_window.resize(PhysicalSize {
            width: monitor.size().width,
            height: 50,
        });
        gl_window.window().set_inner_size(PhysicalSize {
            width: monitor.size().width,
            height: 50,
        });
        gl_window.window().set_min_inner_size(Some(PhysicalSize {
            width: monitor.size().width,
            height: 50,
        }));
        gl_window.window().set_max_inner_size(Some(PhysicalSize {
            width: monitor.size().width,
            height: 50,
        }));
        gl_window.window().set_outer_position(monitor.position());
    }

    (gl_window, gl)
}
