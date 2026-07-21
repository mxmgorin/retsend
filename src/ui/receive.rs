//! The Receive tab: the branded landing screen. The two-tone wordmark hero,
//! who we are on the network, and a "ready / waiting" status. Incoming
//! requests still arrive as the Prompt modal on top of this.

use super::theme;
use egui_sdl2::egui;

/// The `ret` half of the wordmark (app INK, matching the SVG wordmark).
const INK: egui::Color32 = egui::Color32::from_rgb(0xec, 0xec, 0xea);
/// Warm end of the `send` gradient — the brand coral (`brand.py` CORAL).
const SEND_WARM: egui::Color32 = egui::Color32::from_rgb(0xff, 0x8c, 0x69);

/// Everything the Receive renderer needs, snapshotted by `AppUi::update`.
pub struct ReceiveData {
    pub alias: String,
    /// e.g. "HTTPS · 192.168.1.42:53317"
    pub endpoint: String,
    /// Quick-save (auto-accept) is on — shown as a badge under the status.
    pub quick_save: bool,
}

pub fn render(root: &mut egui::Ui, data: &ReceiveData) {
    egui::Panel::bottom("tab_footer").show_inside(root, |ui| {
        ui.add_space(4.0);
        super::home::hint_bar(ui, &[("L1/R1", "Tabs"), ("Select", "Refresh")]);
        ui.add_space(4.0);
    });

    egui::CentralPanel::default().show_inside(root, |ui| {
        const HERO_H: f32 = 190.0; // wordmark + gaps + status block, roughly
        let top = ((ui.available_height() - HERO_H) / 2.0).max(8.0);
        ui.vertical_centered(|ui| {
            ui.add_space(top);
            add_wordmark(ui);
            ui.add_space(24.0);
            ui.label(
                egui::RichText::new(format!("Ready to receive as {}", data.alias))
                    .size(theme::ROW_FONT)
                    .strong(),
            );
            ui.add_space(4.0);
            ui.label(
                egui::RichText::new(&data.endpoint)
                    .size(theme::DETAIL_FONT)
                    .color(theme::DIM),
            );
            ui.add_space(12.0);
            ui.label(
                egui::RichText::new("Waiting for a sender…")
                    .size(theme::DETAIL_FONT)
                    .color(theme::DIM),
            );
            if data.quick_save {
                ui.add_space(4.0);
                ui.label(
                    egui::RichText::new("Quick save on — incoming files auto-accepted")
                        .size(theme::DETAIL_FONT)
                        .color(theme::ACCENT),
                );
            }
        });
    });
}

/// The brand wordmark hero: a large two-tone "ret·send" logotype — "ret" in ink,
/// "send" in the send gradient (teal warming to coral along the top edge, matching
/// the SVG wordmark). Built as one `LayoutJob` on a single centered line with wide
/// letter-spacing.
///
/// egui has no gradient text fill, so we tag the `send` glyphs with a marker color
/// ([`theme::ACCENT`]), tessellate the galley ourselves, and recolor those vertices
/// by height. Ported from retsurf's `ui/home.rs::add_wordmark`.
fn add_wordmark(ui: &mut egui::Ui) {
    const SIZE: f32 = 40.0;
    const TRACKING: f32 = 3.0;
    let mut job = egui::text::LayoutJob::default();
    // Keep the wordmark on Hack (monospace) for its logo feel, like the SVG.
    let fmt = |color: egui::Color32| egui::TextFormat {
        font_id: egui::FontId::monospace(SIZE),
        color,
        extra_letter_spacing: TRACKING,
        ..Default::default()
    };
    job.append("ret", 0.0, fmt(INK));
    // Leading space: epaint skips extra_letter_spacing on a section's first glyph,
    // so the ret/send joint needs it added back to match the other gaps. ACCENT
    // here is a marker recolored to the gradient below.
    job.append("send", TRACKING, fmt(theme::ACCENT));

    let galley = ui.ctx().fonts_mut(|f| f.layout_job(job));
    let (rect, _) = ui.allocate_exact_size(galley.size(), egui::Sense::hover());

    // Tessellate the galley into a mesh and recolor the `send` (marker-colored)
    // vertices by their height: teal over the lower ~55%, warming to coral along
    // the top edge — the same stops as `_send_gradient` in the brand SVGs.
    let ppp = ui.ctx().pixels_per_point();
    let tex = ui.ctx().fonts(|f| f.font_image_size());
    let mut mesh = egui::epaint::Mesh::default();
    let shape = egui::epaint::TextShape::new(rect.min, galley, INK);
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
    ui.painter().add(egui::Shape::mesh(mesh));
}

/// Component-wise sRGB lerp. Fine for a subtle brand tint (no need for linear space).
fn lerp_color(a: egui::Color32, b: egui::Color32, t: f32) -> egui::Color32 {
    let t = t.clamp(0.0, 1.0);
    let m = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round() as u8;
    egui::Color32::from_rgb(m(a.r(), b.r()), m(a.g(), b.g()), m(a.b(), b.b()))
}
