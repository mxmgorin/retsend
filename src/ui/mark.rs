//! The brand mark (the app icon without its plate) as the hero on the landing
//! screens. The PNG is baked into the binary and uploaded to egui as a texture
//! on first paint; the SVG master lives with the other brand assets.

use egui_sdl2::egui;
use std::sync::OnceLock;

/// Hero height (logical px), shared by the Send, Receive and About screens.
pub const HERO_SIZE: f32 = 96.0;

static MARK_PNG: &[u8] = include_bytes!("../../resources/retsend-mark.png");

/// Held for the app's lifetime so egui keeps the texture resident. `None`
/// after a failed decode, so the hero silently drops out rather than retrying.
static TEXTURE: OnceLock<Option<egui::TextureHandle>> = OnceLock::new();

/// Size the mark occupies at `size`: a square.
pub fn measure(size: f32) -> egui::Vec2 {
    egui::vec2(size, size)
}

/// Paint the mark centered at `center`, faded to `alpha` (1.0 = full).
pub fn paint(ui: &egui::Ui, center: egui::Pos2, size: f32, alpha: f32) {
    let Some(tex) = TEXTURE.get_or_init(|| load(ui.ctx())) else {
        return;
    };
    let rect = egui::Rect::from_center_size(center, measure(size));
    let uv = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0));
    let tint = egui::Color32::WHITE.linear_multiply(alpha);
    ui.painter().image(tex.id(), rect, uv, tint);
}

fn load(ctx: &egui::Context) -> Option<egui::TextureHandle> {
    let mut reader = match png::Decoder::new(std::io::Cursor::new(MARK_PNG)).read_info() {
        Ok(r) => r,
        Err(e) => {
            log::warn!("brand mark: PNG header decode failed: {e}");
            return None;
        }
    };
    let mut buf = vec![0u8; reader.output_buffer_size()?];
    let info = match reader.next_frame(&mut buf) {
        Ok(i) => i,
        Err(e) => {
            log::warn!("brand mark: PNG decode failed: {e}");
            return None;
        }
    };
    if info.color_type != png::ColorType::Rgba || info.bit_depth != png::BitDepth::Eight {
        log::warn!(
            "brand mark: unexpected PNG format {:?}/{:?}",
            info.color_type,
            info.bit_depth
        );
        return None;
    }
    let size = [info.width as usize, info.height as usize];
    let image = egui::ColorImage::from_rgba_unmultiplied(size, &buf[..info.buffer_size()]);
    // Mipmapped: the 256px source is drawn at ~96 logical px.
    let options = egui::TextureOptions {
        mipmap_mode: Some(egui::TextureFilter::Linear),
        ..egui::TextureOptions::LINEAR
    };
    Some(ctx.load_texture("brand-mark", image, options))
}
