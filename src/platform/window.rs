//! Window construction: the renderer order this device should be offered, and
//! the icon. Everything else lives in [`egui_sdl2::EguiWindow`], which owns the
//! window and falls through the order until one comes up.

use crate::config::DisplayConfig;
use egui_sdl2::{EguiWindow, Renderer};
use sdl2::Sdl;

/// Configured GL flavour first, then the other, then SDL's renderer — a
/// handheld whose blobs don't match what we ask for gets a slower screen
/// instead of an exit code. `RETSEND_SOFTWARE=1` skips GL entirely.
pub fn open(sdl: &Sdl, config: &DisplayConfig) -> Result<EguiWindow, String> {
    let video_subsystem = sdl.video()?;
    let order: &[Renderer] = if cfg!(target_os = "vita") {
        // VitaSDK's SDL draws through its own GXM renderer and offers no GL.
        &[Renderer::Canvas, Renderer::CanvasBlit]
    } else if blit_first() {
        log::info!("RETSEND_BLIT set; presenting the canvas as a texture copy");
        &[Renderer::CanvasBlit, Renderer::Canvas]
    } else if software_only() {
        log::info!("RETSEND_SOFTWARE set; not asking for GL");
        &[Renderer::Canvas, Renderer::CanvasBlit]
    } else if config.use_gles {
        &[
            Renderer::Gles3,
            Renderer::Gl32,
            Renderer::Canvas,
            Renderer::CanvasBlit,
        ]
    } else {
        &[
            Renderer::Gl32,
            Renderer::Gles3,
            Renderer::Canvas,
            Renderer::CanvasBlit,
        ]
    };

    let size = panel_size(&video_subsystem).unwrap_or((config.width, config.height));
    let mut window = EguiWindow::new(
        &video_subsystem,
        "retsend",
        size,
        |builder| {
            builder.resizable();
        },
        order,
    )?;
    set_window_icon(window.window_mut());

    let (w, h) = window.window().drawable_size();
    log::info!("window: {:?} renderer ({w}x{h})", window.renderer());
    Ok(window)
}

/// The panel's own size, on drivers whose window cannot usefully be another:
/// a Miyoo draws a smaller one centred in a black border (the Flip's panel is
/// 752x560), and the Vita's screen is fixed at 960x544.
fn panel_size(video: &sdl2::VideoSubsystem) -> Option<(u32, u32)> {
    if !matches!(video.current_video_driver(), "Mini" | "mmiyoo" | "VITA") {
        return None;
    }
    // An older bundled SDL leaves the mode zeroed, so treat a small one as no
    // answer rather than opening a window nothing can draw into.
    let mode = video.desktop_display_mode(0).ok()?;
    let (w, h) = (u32::try_from(mode.w).ok()?, u32::try_from(mode.h).ok()?);
    (w >= 320 && h >= 240).then(|| {
        log::info!("panel: {w}x{h} from the video driver");
        (w, h)
    })
}

fn software_only() -> bool {
    std::env::var("RETSEND_SOFTWARE").is_ok_and(|v| v != "0")
}

/// The Miyoo Mini's panel driver shows texture copies and drops everything else
/// without erroring, so the launcher has to ask: a blank screen can't be detected.
fn blit_first() -> bool {
    std::env::var("RETSEND_BLIT").is_ok_and(|v| v != "0")
}

/// Set the window icon from the bundled PNG (RGBA8), baked into the binary.
/// Best-effort: any decode failure just logs and leaves SDL's default.
/// Bare-kmsdrm has no window-icon concept, so SDL no-ops there harmlessly.
fn set_window_icon(window: &mut sdl2::video::Window) {
    use sdl2::pixels::PixelFormatEnum;
    static ICON_PNG: &[u8] = include_bytes!("../../resources/icon/icon.png");

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
