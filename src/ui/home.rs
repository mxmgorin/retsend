//! The radar screen: our identity up top, nearby devices in the middle,
//! button hints at the bottom.

use super::theme;
use egui_sdl2::egui;

/// The `ret` half of the wordmark (app INK, matching the SVG wordmark).
const INK: egui::Color32 = egui::Color32::from_rgb(0xec, 0xec, 0xea);
/// Warm end of the `send` gradient — the brand coral (`brand.py` CORAL).
const SEND_WARM: egui::Color32 = egui::Color32::from_rgb(0xff, 0x8c, 0x69);

/// A display-ready radar row. `AppUi::update` builds these (from the peer
/// registry once discovery exists), keeping this renderer decoupled from the
/// net layer.
pub struct PeerRow {
    pub alias: String,
    /// Shown under the alias, e.g. "Pixel · 192.168.1.23".
    pub detail: String,
    /// Transport badge on the right: "HTTP" / "HTTPS".
    pub proto: String,
}

/// Everything the renderer needs, snapshotted by `AppUi::update` outside the
/// egui closure (shared-state locks must not be held while egui borrows `self`).
pub struct HomeData {
    pub alias: String,
    /// e.g. "HTTP · 192.168.1.42:53317"
    pub endpoint: String,
    pub peers: Vec<PeerRow>,
    pub cursor: Option<usize>,
}

pub fn render(root: &mut egui::Ui, data: &HomeData) {
    egui::Panel::top("home_header").show_inside(root, |ui| {
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new(&data.alias)
                    .size(theme::ROW_FONT + 2.0)
                    .strong(),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(
                    egui::RichText::new(&data.endpoint)
                        .size(theme::DETAIL_FONT)
                        .color(theme::DIM),
                );
            });
        });
        ui.add_space(6.0);
    });

    egui::Panel::bottom("home_footer").show_inside(root, |ui| {
        ui.add_space(4.0);
        hint_bar(
            ui,
            &[("A", "Send"), ("Select", "Refresh"), ("Start", "Settings")],
        );
        ui.add_space(4.0);
    });

    egui::CentralPanel::default().show_inside(root, |ui| {
        if data.peers.is_empty() {
            // Idle radar: brand the empty screen with the two-tone wordmark hero
            // (retsurf's start-page treatment) so it reads as branded, not blank.
            const HERO_H: f32 = 150.0; // wordmark + gap + hint block, roughly
            let top = ((ui.available_height() - HERO_H) / 2.0).max(8.0);
            ui.vertical_centered(|ui| {
                ui.add_space(top);
                add_wordmark(ui);
                ui.add_space(24.0);
                ui.label(
                    egui::RichText::new(
                        "Searching for devices…\n\nOpen LocalSend on your phone or PC\non the same network.",
                    )
                    .size(theme::ROW_FONT)
                    .color(theme::DIM),
                );
            });
            return;
        }

        egui::ScrollArea::vertical().show(ui, |ui| {
            for (i, peer) in data.peers.iter().enumerate() {
                let selected = data.cursor == Some(i);
                let row = peer_row(ui, peer, selected);
                if selected {
                    row.scroll_to_me(None);
                }
            }
        });
    });
}

fn peer_row(ui: &mut egui::Ui, peer: &PeerRow, selected: bool) -> egui::Response {
    let desired = egui::vec2(ui.available_width(), theme::ROW_HEIGHT);
    let (rect, response) = ui.allocate_exact_size(desired, egui::Sense::hover());
    if selected {
        ui.painter().rect(
            rect,
            6.0,
            theme::ACCENT.linear_multiply(0.30),
            egui::Stroke::new(1.0, theme::ACCENT),
            egui::StrokeKind::Inside,
        );
    }
    let padding = 10.0;
    let painter = ui.painter();
    painter.text(
        rect.left_top() + egui::vec2(padding, 7.0),
        egui::Align2::LEFT_TOP,
        &peer.alias,
        egui::FontId::proportional(theme::ROW_FONT),
        ui.visuals().text_color(),
    );
    painter.text(
        rect.left_bottom() + egui::vec2(padding, -7.0),
        egui::Align2::LEFT_BOTTOM,
        &peer.detail,
        egui::FontId::proportional(theme::DETAIL_FONT),
        theme::DIM,
    );
    // Transport badge: peers announcing https need the HTTPS milestone before
    // we can send to them; make that visible early.
    painter.text(
        rect.right_center() - egui::vec2(padding, 0.0),
        egui::Align2::RIGHT_CENTER,
        &peer.proto,
        egui::FontId::proportional(theme::DETAIL_FONT),
        theme::DIM,
    );
    response
}

/// The brand wordmark hero: a large two-tone "ret·send" logotype — "ret" in ink,
/// "send" in the send gradient (teal warming to coral along the top edge, matching
/// the SVG wordmark) — so the idle radar reads as branded rather than blank. Built
/// as one `LayoutJob` on a single centered line with wide letter-spacing.
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
    for v in mesh.vertices.iter_mut().filter(|v| v.color == theme::ACCENT) {
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

/// `[Btn] Action · [Btn] Action` bar shared by the screens.
pub fn hint_bar(ui: &mut egui::Ui, hints: &[(&str, &str)]) {
    ui.horizontal(|ui| {
        for (i, (button, action)) in hints.iter().enumerate() {
            if i > 0 {
                ui.label(egui::RichText::new("·").color(theme::DIM));
            }
            ui.label(
                egui::RichText::new(*button)
                    .size(theme::DETAIL_FONT)
                    .strong()
                    .color(theme::ACCENT),
            );
            ui.label(
                egui::RichText::new(*action)
                    .size(theme::DETAIL_FONT)
                    .color(theme::DIM),
            );
        }
    });
}
