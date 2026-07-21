//! The window plus the single GL/GLES context that SDL2 owns. Retsurf's
//! `AppWindow` minus the Servo rendering context — egui (`glow`) is the only
//! GL client here, painting straight into the default framebuffer.

use crate::config::DisplayConfig;
use sdl2::video::{GLContext, GLProfile};
use sdl2::Sdl;
use std::sync::Arc;

pub struct AppWindow {
    _video_subsystem: sdl2::VideoSubsystem,
    window: sdl2::video::Window,
    // Kept alive for the lifetime of the window; dropping it destroys the context.
    _gl_context: GLContext,
    glow_ctx: Arc<glow::Context>,
}

impl AppWindow {
    pub fn new(sdl: &Sdl, config: &DisplayConfig) -> Result<Self, String> {
        let video_subsystem = sdl.video()?;

        {
            let gl_attr = video_subsystem.gl_attr();
            if config.use_gles {
                // Mali blobs on RK3326/RK3566 expose GLES 3.x; egui_glow is
                // happy with 3.0.
                gl_attr.set_context_profile(GLProfile::GLES);
                gl_attr.set_context_version(3, 0);
            } else {
                gl_attr.set_context_profile(GLProfile::Core);
                gl_attr.set_context_version(3, 2);
            }
            gl_attr.set_double_buffer(true);
        }

        let mut window = video_subsystem
            .window("retsend", config.width, config.height)
            .opengl()
            .resizable()
            .build()
            .map_err(|e| format!("failed to build window: {e}"))?;

        set_window_icon(&mut window);

        let gl_context = window
            .gl_create_context()
            .map_err(|e| format!("failed to create GL context: {e}"))?;
        window
            .gl_make_current(&gl_context)
            .map_err(|e| format!("failed to make GL context current: {e}"))?;

        // Cap the main loop to the display refresh; without this the loop
        // would busy-spin while a held D-pad drives navigation repeats.
        let _ = video_subsystem.gl_set_swap_interval(sdl2::video::SwapInterval::VSync);

        let get_proc =
            |name: &str| video_subsystem.gl_get_proc_address(name) as *const std::os::raw::c_void;
        let glow_ctx = Arc::new(unsafe { glow::Context::from_loader_function(get_proc) });

        let (w, h) = window.drawable_size();
        log::info!("window: GL context current ({w}x{h})");

        Ok(Self {
            _video_subsystem: video_subsystem,
            window,
            _gl_context: gl_context,
            glow_ctx,
        })
    }

    pub fn sdl2_window(&self) -> &sdl2::video::Window {
        &self.window
    }

    pub fn glow_ctx(&self) -> Arc<glow::Context> {
        self.glow_ctx.clone()
    }

    #[inline]
    pub fn present(&self) {
        self.window.gl_swap_window();
    }
}

/// Set the window icon from the bundled PNG (RGBA8), baked into the binary.
/// Best-effort: any decode failure just logs and leaves SDL's default.
/// Bare-kmsdrm has no window-icon concept, so SDL no-ops there harmlessly.
fn set_window_icon(window: &mut sdl2::video::Window) {
    use sdl2::pixels::PixelFormatEnum;
    static ICON_PNG: &[u8] = include_bytes!("../../resources/icon.png");

    let mut reader = match png::Decoder::new(std::io::Cursor::new(ICON_PNG)).read_info() {
        Ok(r) => r,
        Err(e) => return log::warn!("window icon: PNG header decode failed: {e}"),
    };
    let buf_size = match reader.output_buffer_size() {
        Some(n) => n,
        None => return log::warn!("window icon: PNG output buffer size overflow"),
    };
    let mut buf = vec![0u8; buf_size];
    let info = match reader.next_frame(&mut buf) {
        Ok(i) => i,
        Err(e) => return log::warn!("window icon: PNG decode failed: {e}"),
    };
    if info.color_type != png::ColorType::Rgba || info.bit_depth != png::BitDepth::Eight {
        return log::warn!(
            "window icon: unexpected PNG format {:?}/{:?}",
            info.color_type,
            info.bit_depth
        );
    }
    // png emits R,G,B,A byte order, which is ABGR8888 on our (little-endian) targets.
    let frame = &mut buf[..info.line_size * info.height as usize];
    let surface = sdl2::surface::Surface::from_data(
        frame,
        info.width,
        info.height,
        info.line_size as u32,
        PixelFormatEnum::ABGR8888,
    );
    match surface {
        // SDL_SetWindowIcon copies the pixels, so the temporary surface can drop.
        Ok(surface) => window.set_icon(surface),
        Err(e) => log::warn!("window icon: surface build failed: {e}"),
    }
}
