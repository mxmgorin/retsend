//! The two-tone "ret·send" logotype: "ret" in ink, "send" in a teal→coral
//! gradient. Hero on Receive, faded watermark on Send.
//!
//! egui has no gradient text fill, so we mark the `send` glyphs with
//! [`theme::ACCENT`], tessellate the galley, and recolor by height.

use super::theme;
use egui_sdl2::egui;

/// The `ret` half (app INK, matching the SVG wordmark).
const INK: egui::Color32 = egui::Color32::from_rgb(0xec, 0xec, 0xea);
/// Warm end of the `send` gradient — the brand coral (`brand.py` CORAL).
const SEND_WARM: egui::Color32 = egui::Color32::from_rgb(0xff, 0x8c, 0x69);
/// Letter-spacing, also the ret/send joint gap (epaint drops a section's first
/// glyph's spacing, so it's re-added as `send`'s leading space).
const TRACKING: f32 = 3.0;

/// Hero size (logical px), shared by the Send and Receive tabs.
pub const HERO_SIZE: f32 = 40.0;

// The "send" swoosh below the logotype, as fractions of the logotype width `w`.
/// Rise of the arrowhead end above the tail.
const ARROW_DROP: f32 = 0.028;
/// Ribbon half-width, tapering linearly from a heavy tail to a thin head.
const ARROW_HALF_TAIL: f32 = 0.009;
const ARROW_HALF_HEAD: f32 = 0.0013;
/// Head half-width floor (logical px) so the thin end stays a crisp hairline.
const ARROW_MIN_HALF: f32 = 0.7;
/// Arrowhead chevron line width, length, and half-height.
const ARROW_HEAD_STROKE: f32 = 0.005;
const ARROW_HEAD_LEN: f32 = 0.020;
const ARROW_HEAD_HALF: f32 = 0.0105;
/// Clearance below the text box before the swoosh, as a fraction of font size.
const ARROW_GAP: f32 = 0.10;

/// Total vertical room the swoosh needs below the text box.
fn arrow_band(w: f32, size: f32) -> f32 {
    ARROW_GAP * size + ARROW_DROP * w + ARROW_HALF_TAIL * w
}

fn layout(size: f32) -> egui::text::LayoutJob {
    let mut job = egui::text::LayoutJob::default();
    // Monospace, for the logo feel of the SVG.
    let fmt = |color: egui::Color32| egui::TextFormat {
        font_id: egui::FontId::monospace(size),
        color,
        extra_letter_spacing: TRACKING,
        ..Default::default()
    };
    job.append("ret", 0.0, fmt(INK));
    // ACCENT is a marker recolored to the gradient in `paint`.
    job.append("send", TRACKING, fmt(theme::ACCENT));
    job
}

/// Size the wordmark occupies at `size` (text plus the swoosh band below).
pub fn measure(ui: &egui::Ui, size: f32) -> egui::Vec2 {
    let text = ui.ctx().fonts_mut(|f| f.layout_job(layout(size))).size();
    egui::vec2(text.x, text.y + arrow_band(text.x, size))
}

/// Paint the wordmark centered at `center`, faded to `alpha` (1.0 = the full
/// hero, lower = a background watermark).
pub fn paint(ui: &egui::Ui, center: egui::Pos2, size: f32, alpha: f32) {
    let galley = ui.ctx().fonts_mut(|f| f.layout_job(layout(size)));
    // Center text + arrow as one block; text on top, swoosh in the band below.
    let gsize = galley.size();
    let total = egui::vec2(gsize.x, gsize.y + arrow_band(gsize.x, size));
    let min = center - total / 2.0;

    // Tessellate the galley, then recolor the marked `send` vertices by height:
    // teal over the lower ~55%, warming to coral above (the SVG's gradient).
    let ppp = ui.ctx().pixels_per_point();
    let tex = ui.ctx().fonts(|f| f.font_image_size());
    let mut mesh = egui::epaint::Mesh::default();
    let shape = egui::epaint::TextShape::new(min, galley, INK);
    let opts = egui::epaint::TessellationOptions::default();
    egui::epaint::Tessellator::new(ppp, opts, tex, Vec::new()).tessellate_text(&shape, &mut mesh);

    // Key the gradient to the x-height, not the full span: `d`'s ascender would
    // otherwise leave the other glyph tops only partway to coral. Clamping above
    // the x-height makes every glyph top match (the SVG's y2=1120 stop).
    let ys: Vec<f32> = mesh
        .vertices
        .iter()
        .filter(|v| v.color == theme::ACCENT)
        .map(|v| v.pos.y)
        .collect();
    let bot = ys.iter().copied().fold(f32::NEG_INFINITY, f32::max);
    // Each glyph is one quad (4 verts); the x-height is the lowest glyph top
    // (the tall `d` is excluded by taking the max).
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
    // Fade for the watermark.
    if alpha < 1.0 {
        for v in mesh.vertices.iter_mut() {
            v.color = v.color.linear_multiply(alpha);
        }
    }
    ui.painter().add(egui::Shape::mesh(mesh));

    // The swoosh, under the text box and spanning its width.
    arrow(ui.painter(), min.x, min.y + gsize.y, gsize.x, size, alpha);
}

/// The swoosh: an up-curving ribbon of width `w`, heavy tail on the left
/// tapering to a thin head that meets the arrowhead. `left`/`base_y` are the
/// top-left of the band below the text.
fn arrow(painter: &egui::Painter, left: f32, base_y: f32, w: f32, size: f32, alpha: f32) {
    let color = theme::ACCENT.linear_multiply(alpha);
    let drop = ARROW_DROP * w;
    let x_l = left;
    let x_r = left + w;
    let y_r = base_y + ARROW_GAP * size; // head end, riding high
    let y_l = y_r + drop; // tail end, sitting low
    let half_tail = ARROW_HALF_TAIL * w;
    let half_head = ARROW_HALF_HEAD * w;

    // Ease-out centerline (steep at the tail, flattening toward the head),
    // widened into a ribbon that tapers from the heavy tail to the thin head.
    // The curve is shallow (~3°), so edges are offset vertically, not by normal.
    const N: usize = 32;
    let mut top = Vec::with_capacity(N);
    let mut bot = Vec::with_capacity(N);
    for i in 0..N {
        let u = i as f32 / (N - 1) as f32;
        let x = x_l + (x_r - x_l) * u;
        let cy = y_l - drop * (1.0 - (1.0 - u).powi(2));
        let h = (half_tail + (half_head - half_tail) * u).max(ARROW_MIN_HALF);
        top.push(egui::pos2(x, cy - h));
        bot.push(egui::pos2(x, cy + h));
    }

    // Fill as a triangle strip (egui can't fill a concave outline), then
    // anti-alias the boundary with a hairline stroke — a raw mesh has hard edges.
    let mut mesh = egui::epaint::Mesh::default();
    let uv = egui::epaint::WHITE_UV;
    for i in 0..N {
        let base = mesh.vertices.len() as u32;
        mesh.vertices.push(egui::epaint::Vertex { pos: top[i], uv, color });
        mesh.vertices.push(egui::epaint::Vertex { pos: bot[i], uv, color });
        if i > 0 {
            mesh.indices
                .extend_from_slice(&[base - 2, base - 1, base, base, base - 1, base + 1]);
        }
    }
    painter.add(egui::Shape::mesh(mesh));
    let outline: Vec<egui::Pos2> = top.iter().chain(bot.iter().rev()).copied().collect();
    painter.add(egui::Shape::closed_line(outline, egui::Stroke::new(1.0, color)));
    // Rounded left cap, flush with the ribbon (radius == tail half-width).
    painter.circle_filled(egui::pos2(x_l, y_l), half_tail, color);

    // Arrowhead: a `>` chevron whose tip meets the thin head.
    let head_stroke = egui::Stroke::new((ARROW_HEAD_STROKE * w).max(1.3), color);
    let tip = egui::pos2(x_r, y_r);
    let back = egui::vec2(ARROW_HEAD_LEN * w, ARROW_HEAD_HALF * w);
    painter.add(egui::Shape::line(
        vec![tip - back, tip, egui::pos2(tip.x - back.x, tip.y + back.y)],
        head_stroke,
    ));
}

/// Component-wise sRGB lerp — fine for a subtle brand tint.
fn lerp_color(a: egui::Color32, b: egui::Color32, t: f32) -> egui::Color32 {
    let t = t.clamp(0.0, 1.0);
    let m = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round() as u8;
    egui::Color32::from_rgb(m(a.r(), b.r()), m(a.g(), b.g()), m(a.b(), b.b()))
}
