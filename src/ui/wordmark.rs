//! The two-tone "ret·send" brand logotype — "ret" in ink, "send" in the send
//! gradient (teal warming to coral along the top edge, matching the SVG
//! wordmark). Used full-strength as the Receive hero and faded as a background
//! watermark on the Send tab.
//!
//! egui has no gradient text fill, so we tag the `send` glyphs with a marker
//! color ([`theme::ACCENT`]), tessellate the galley ourselves, and recolor those
//! vertices by height. Ported from retsurf's `ui/home.rs::add_wordmark`.

use super::theme;
use egui_sdl2::egui;

/// The `ret` half (app INK, matching the SVG wordmark).
const INK: egui::Color32 = egui::Color32::from_rgb(0xec, 0xec, 0xea);
/// Warm end of the `send` gradient — the brand coral (`brand.py` CORAL).
const SEND_WARM: egui::Color32 = egui::Color32::from_rgb(0xff, 0x8c, 0x69);
/// Letter-spacing; also the ret/send joint gap (epaint skips a section's first
/// glyph's spacing, so it's re-added as `send`'s leading space).
const TRACKING: f32 = 3.0;

/// Hero size (logical px) — the branded landing treatment shared by the Send
/// and Receive tabs.
pub const HERO_SIZE: f32 = 40.0;

fn layout(size: f32) -> egui::text::LayoutJob {
    let mut job = egui::text::LayoutJob::default();
    // Keep the wordmark on Hack (monospace) for its logo feel, like the SVG.
    let fmt = |color: egui::Color32| egui::TextFormat {
        font_id: egui::FontId::monospace(size),
        color,
        extra_letter_spacing: TRACKING,
        ..Default::default()
    };
    job.append("ret", 0.0, fmt(INK));
    // ACCENT here is a marker recolored to the gradient in `paint`.
    job.append("send", TRACKING, fmt(theme::ACCENT));
    job
}

/// The size the wordmark occupies at `size` — for reserving layout space.
pub fn measure(ui: &egui::Ui, size: f32) -> egui::Vec2 {
    ui.ctx().fonts_mut(|f| f.layout_job(layout(size))).size()
}

/// Paint the wordmark centered at `center`, faded to `alpha` (1.0 = the full
/// hero, lower = a background watermark).
pub fn paint(ui: &egui::Ui, center: egui::Pos2, size: f32, alpha: f32) {
    let galley = ui.ctx().fonts_mut(|f| f.layout_job(layout(size)));
    let min = center - galley.size() / 2.0;

    // Tessellate the galley into a mesh and recolor the `send` (marker-colored)
    // vertices by their height: teal over the lower ~55%, warming to coral along
    // the top edge — the same stops as `_send_gradient` in the brand SVGs.
    let ppp = ui.ctx().pixels_per_point();
    let tex = ui.ctx().fonts(|f| f.font_image_size());
    let mut mesh = egui::epaint::Mesh::default();
    let shape = egui::epaint::TextShape::new(min, galley, INK);
    let opts = egui::epaint::TessellationOptions::default();
    egui::epaint::Tessellator::new(ppp, opts, tex, Vec::new()).tessellate_text(&shape, &mut mesh);

    // Key the gradient to the x-height, not the full vertex span: `d`'s ascender
    // reaches higher than the other glyphs, so spanning to it would leave the
    // x-height tops only partway to coral. Anchoring at the x-height clamps
    // everything above it to coral, so every glyph's top matches — same as the
    // SVG's y2=1120 stop.
    let ys: Vec<f32> = mesh
        .vertices
        .iter()
        .filter(|v| v.color == theme::ACCENT)
        .map(|v| v.pos.y)
        .collect();
    let bot = ys.iter().copied().fold(f32::NEG_INFINITY, f32::max);
    // Each glyph is one quad (4 verts); the x-height is the lowest of the glyph
    // tops (the tall `d` top is the outlier and is excluded by taking the max).
    let x_top = ys
        .chunks(4)
        .map(|g| g.iter().copied().fold(f32::INFINITY, f32::min))
        .fold(f32::NEG_INFINITY, f32::max);
    let span = (bot - x_top).max(1.0);
    for v in mesh
        .vertices
        .iter_mut()
        .filter(|v| v.color == theme::ACCENT)
    {
        let frac = ((bot - v.pos.y) / span).clamp(0.0, 1.0); // 0 baseline, 1 x-height+
        let t = ((frac - 0.55) / 0.45).clamp(0.0, 1.0);
        v.color = lerp_color(theme::ACCENT, SEND_WARM, t);
    }
    // Fade the whole mesh for the watermark use.
    if alpha < 1.0 {
        for v in mesh.vertices.iter_mut() {
            v.color = v.color.linear_multiply(alpha);
        }
    }
    ui.painter().add(egui::Shape::mesh(mesh));
}

/// Component-wise sRGB lerp. Fine for a subtle brand tint (no need for linear space).
fn lerp_color(a: egui::Color32, b: egui::Color32, t: f32) -> egui::Color32 {
    let t = t.clamp(0.0, 1.0);
    let m = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round() as u8;
    egui::Color32::from_rgb(m(a.r(), b.r()), m(a.g(), b.g()), m(a.b(), b.b()))
}
