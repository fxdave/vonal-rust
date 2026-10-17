use std::cell::Cell;

use egui_glow::glow;
use glutin::config::ConfigTemplateBuilder;
use glutin::context::{ContextApi, ContextAttributesBuilder, Version};
use glutin::display::GetGlDisplay;
use glutin::prelude::*;
use glutin::surface::GlSurface;
use glutin_winit::GlWindow;
use raw_window_handle::{
    HasRawDisplayHandle, HasRawWindowHandle, RawDisplayHandle, RawWindowHandle,
};
use winit::dpi::PhysicalSize;
use winit::event_loop::{EventLoop, EventLoopWindowTarget};
use winit::platform::wayland::WindowBuilderExtWayland;
use winit::window::WindowBuilder;
use x11::xlib::_XDisplay;

pub fn create_display<TUserEvent>(
    event_loop: &EventLoop<TUserEvent>,
) -> (GlutinWindowContext, egui_glow::painter::Context) {
    let window_builder = WindowBuilder::new()
        .with_visible(false)
        .with_decorations(false)
        .with_resizable(false)
        .with_window_level(winit::window::WindowLevel::AlwaysOnTop)
        .with_inner_size(PhysicalSize {
            width: 10,
            height: 10,
        })
        .with_transparent(true)
        .with_title("Vonal")
        .with_name("vonal", "vonal");

    let template_builder = ConfigTemplateBuilder::new().with_alpha_size(8);

    let (window, gl_config) = glutin_winit::DisplayBuilder::new()
        .with_window_builder(Some(window_builder.clone()))
        .build(event_loop, template_builder, |configs| {
            // Find the config with the maximum number of samples
            configs
                .reduce(|acc, current| {
                    let is_current_transparent = current.supports_transparency().unwrap_or(false);
                    let is_acc_transparent = acc.supports_transparency().unwrap_or(false);
                    let gain_bigger_samples = current.num_samples() > acc.num_samples();

                    let case_transparency_is_not_supported =
                        !is_acc_transparent && !is_current_transparent && gain_bigger_samples;
                    let case_transparency_is_supported =
                        is_acc_transparent && is_current_transparent && gain_bigger_samples;
                    let case_transparency_become_supported =
                        !is_acc_transparent && is_current_transparent;
                    let is_current_better = case_transparency_become_supported
                        || case_transparency_is_supported
                        || case_transparency_is_not_supported;

                    if is_current_better {
                        current
                    } else {
                        acc
                    }
                })
                .unwrap()
        })
        .unwrap();

    let window = window.unwrap();
    let gl_window = unsafe { GlutinWindowContext::new(window, window_builder, gl_config) };

    let gl = unsafe {
        glow::Context::from_loader_function(|s| {
            let s = std::ffi::CString::new(s)
                .expect("failed to construct C string from string for gl proc address");

            gl_window.gl_display.get_proc_address(&s)
        })
    };

    (gl_window, gl)
}

pub struct GlutinWindowContext {
    pub window: winit::window::Window,
    pub gl_context: glutin::context::PossiblyCurrentContext,
    pub gl_display: glutin::display::Display,
    pub gl_surface: glutin::surface::Surface<glutin::surface::WindowSurface>,
    gl_config: glutin::config::Config,
    window_builder: WindowBuilder,
    is_visible: Cell<bool>,
    is_presented: Cell<bool>,
}

impl GlutinWindowContext {
    #[allow(unsafe_code)]
    unsafe fn new(
        winit_window: winit::window::Window,
        window_builder: WindowBuilder,
        config: glutin::config::Config,
    ) -> Self {
        let raw_window_handle = winit_window.raw_window_handle();
        let context_attributes =
            glutin::context::ContextAttributesBuilder::new().build(Some(raw_window_handle));

        // Since glutin by default tries to create OpenGL core context, which may not be
        // present we should try gles.
        let fallback_context_attributes = ContextAttributesBuilder::new()
            .with_context_api(ContextApi::Gles(None))
            .build(Some(raw_window_handle));

        // There are also some old devices that support neither modern OpenGL nor GLES.
        // To support these we can try and create a 2.1 context.
        let legacy_context_attributes = ContextAttributesBuilder::new()
            .with_context_api(ContextApi::OpenGl(Some(Version::new(2, 1))))
            .build(Some(raw_window_handle));

        let gl_display = config.display();
        let gl_context_candidate = unsafe {
            gl_display
                .create_context(&config, &context_attributes)
                .unwrap_or_else(|_| {
                    gl_display
                        .create_context(&config, &fallback_context_attributes)
                        .unwrap_or_else(|_| {
                            gl_display
                                .create_context(&config, &legacy_context_attributes)
                                .expect("failed to create context")
                        })
                })
        };

        let surface_attributes = winit_window.build_surface_attributes(Default::default());
        let gl_surface = gl_display
            .create_window_surface(&config, &surface_attributes)
            .unwrap();

        let gl_context = gl_context_candidate.make_current(&gl_surface).unwrap();
        set_swap_interval(&gl_context, &gl_surface);

        GlutinWindowContext {
            window: winit_window,
            gl_context,
            gl_display,
            gl_surface,
            gl_config: config,
            window_builder,
            is_visible: Cell::new(false),
            is_presented: Cell::new(false),
        }
    }

    pub fn window(&self) -> &winit::window::Window {
        &self.window
    }

    pub fn resize(&self, physical_size: winit::dpi::PhysicalSize<u32>) {
        self.gl_surface.resize(
            &self.gl_context,
            physical_size.width.try_into().unwrap(),
            physical_size.height.try_into().unwrap(),
        );
    }

    pub fn swap_buffers(&self) -> glutin::error::Result<()> {
        self.is_presented.set(true);
        self.gl_surface.swap_buffers(&self.gl_context)
    }

    pub fn is_wayland(&self) -> bool {
        matches!(
            self.window.raw_display_handle(),
            RawDisplayHandle::Wayland(_)
        )
    }

    pub fn is_visible(&self) -> bool {
        self.window
            .is_visible()
            .unwrap_or_else(|| self.is_visible.get())
    }

    pub fn set_visible(&self, visible: bool) {
        self.is_visible.set(visible);
        self.window.set_visible(visible);
        if visible {
            self.window.request_redraw();
        }
    }

    /// Wayland can't hide a window, so the presented one is replaced
    /// with a new window that stays unmapped until it's drawn again.
    pub fn recreate_hidden_window<TUserEvent>(
        &mut self,
        event_loop: &EventLoopWindowTarget<TUserEvent>,
    ) {
        if !self.is_wayland() || self.is_visible.get() || !self.is_presented.get() {
            return;
        }

        let window =
            glutin_winit::finalize_window(event_loop, self.window_builder.clone(), &self.gl_config)
                .unwrap();
        let surface_attributes = window.build_surface_attributes(Default::default());
        let gl_surface = unsafe {
            self.gl_display
                .create_window_surface(&self.gl_config, &surface_attributes)
                .unwrap()
        };
        self.gl_context.make_current(&gl_surface).unwrap();
        set_swap_interval(&self.gl_context, &gl_surface);

        // the old surface has to be dropped before its window
        drop(std::mem::replace(&mut self.gl_surface, gl_surface));
        self.window = window;
        self.is_presented.set(false);
    }

    pub fn get_focused_monitor(&self) -> Option<winit::monitor::MonitorHandle> {
        if self.is_wayland() {
            // Wayland doesn't expose the global pointer position
            return self
                .window
                .current_monitor()
                .or_else(|| self.window.available_monitors().next());
        }

        let pointer = self.query_pointer();
        let mut monitors = self.window.available_monitors().filter(|monitor| {
            let position = monitor.position();
            let size = monitor.size();

            let x_ok = position.x <= pointer.0 && pointer.0 < (position.x + size.width as i32);
            let y_ok = position.y <= pointer.1 && pointer.1 < (position.y + size.height as i32);

            x_ok && y_ok
        });

        monitors.next()
    }

    fn query_pointer(&self) -> (i32, i32) {
        let raw_window_handle = self.window().raw_window_handle();
        let raw_display_handle = self.window().raw_display_handle();
        let connection = if let RawDisplayHandle::Xlib(raw) = raw_display_handle {
            raw.display
        } else {
            panic!("(Connection) We only support X.org over Xlib")
        };
        let window = if let RawWindowHandle::Xlib(raw) = raw_window_handle {
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
}

fn set_swap_interval(
    gl_context: &glutin::context::PossiblyCurrentContext,
    gl_surface: &glutin::surface::Surface<glutin::surface::WindowSurface>,
) {
    gl_surface
        .set_swap_interval(
            gl_context,
            glutin::surface::SwapInterval::Wait(std::num::NonZeroU32::new(1).unwrap()),
        )
        .unwrap();
}
