use egui_glow::glow;
use raw_window_handle::{
    HasRawDisplayHandle, HasRawWindowHandle, RawDisplayHandle, RawWindowHandle,
};
use winit::dpi::PhysicalSize;
use winit::event_loop::EventLoop;
use winit::window::WindowBuilder;
use x11::xlib::_XDisplay;

pub fn create_display<TUserEvent>(
    event_loop: &EventLoop<TUserEvent>,
) -> (GlutinWindowContext, egui_glow::painter::Context) {
    let winit_window = WindowBuilder::new()
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

        // This was GlxThenEgl, but it broke
        #[cfg(target_os = "linux")]
        let preference = glutin::display::DisplayApiPreference::Egl;

        let gl_display = glutin::display::Display::new(raw_display_handle, preference).unwrap();

        let config_template = glutin::config::ConfigTemplateBuilder::new()
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

    pub fn window(&self) -> &winit::window::Window {
        &self.window
    }

    pub fn resize(&self, physical_size: winit::dpi::PhysicalSize<u32>) {
        use glutin::surface::GlSurface;
        self.gl_surface.resize(
            &self.gl_context,
            physical_size.width.try_into().unwrap(),
            physical_size.height.try_into().unwrap(),
        );
    }

    pub fn swap_buffers(&self) -> glutin::error::Result<()> {
        use glutin::surface::GlSurface;
        self.gl_surface.swap_buffers(&self.gl_context)
    }

    fn get_proc_address(&self, addr: &std::ffi::CStr) -> *const std::ffi::c_void {
        use glutin::display::GlDisplay;
        self.gl_display.get_proc_address(addr)
    }

    pub fn get_focused_monitor(&self) -> Option<winit::monitor::MonitorHandle> {
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
