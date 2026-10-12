//! The egui visual theme: the brand palette, sizing constants for 640×480
//! gamepad-first screens, and the painters every screen shares. Same shape as
//! retsurf's theme so the two apps feel like siblings.
//!
//! The palette is the icon's: a striped sun (amber into coral) inside a teal
//! orbit. Teal marks what comes in and what is selected; coral marks what goes
//! out. Everything else is the dark plate the icon sits on.

use egui_sdl2::egui;

/// The brand accent (teal): incoming transfers, selection, links.
pub const ACCENT: egui::Color32 = egui::Color32::from_rgb(0x3f, 0xb8, 0xa0);
/// The sun's coral: outgoing transfers.
pub const WARM: egui::Color32 = egui::Color32::from_rgb(0xff, 0x8c, 0x69);
/// The sun's amber top, and the orbit arrow's head.
pub const SUN_TOP: egui::Color32 = egui::Color32::from_rgb(0xff, 0xb8, 0x70);

/// The plate the icon sits on; the window clear color.
pub const BG: egui::Color32 = egui::Color32::from_rgb(0x16, 0x17, 0x1a);
/// Header and footer bars, cards, keys: one step up from the plate.
pub const PANEL_FILL: egui::Color32 = egui::Color32::from_rgb(0x1b, 0x1c, 0x20);
/// Raised controls: keys, a switch that is off.
pub const SURFACE: egui::Color32 = egui::Color32::from_rgb(0x24, 0x26, 0x2b);
/// Hairlines: card edges, progress tracks, section rules.
pub const BORDER: egui::Color32 = egui::Color32::from_rgb(0x2e, 0x31, 0x37);
/// Primary text, matching the wordmark.
pub const INK: egui::Color32 = egui::Color32::from_rgb(0xec, 0xec, 0xea);
/// Secondary text: hints, device details, statuses.
pub const DIM: egui::Color32 = egui::Color32::from_rgb(0x8f, 0x94, 0x9c);
/// Backdrop under a modal: what it covers reads as inactive.
pub const SCRIM: egui::Color32 = egui::Color32::from_black_alpha(160);
/// Failure red: declined, failed, no network, seconds running out.
pub const DANGER: egui::Color32 = egui::Color32::from_rgb(0xe0, 0x5a, 0x5a);

/// [`BG`] as the GL clear color.
pub const BACKGROUND: [f32; 4] = [
    0x16 as f32 / 255.0,
    0x17 as f32 / 255.0,
    0x1a as f32 / 255.0,
    1.0,
];

/// Primary row font size (logical px) — readable on a 3.5" 640×480 panel.
pub const ROW_FONT: f32 = 16.0;
/// Secondary line under a row title.
pub const DETAIL_FONT: f32 = 12.5;
/// Screen titles in the top panel.
pub const TITLE_FONT: f32 = ROW_FONT + 2.0;
/// Minimum height of a navigable row.
pub const ROW_HEIGHT: f32 = 44.0;
/// Inset of a row's text from its edge.
pub const ROW_PAD: f32 = 10.0;
/// Corner radius of a row highlight.
pub const ROW_RADIUS: f32 = 6.0;
/// Side of an inline icon beside row text.
pub const ICON_SIZE: f32 = 15.0;

/// Alpha of the wash behind a selected row: tints without swamping the text.
const HIGHLIGHT_WASH: f32 = 0.30;

/// Teal for what comes in, coral for what goes out.
pub fn flow_color(outgoing: bool) -> egui::Color32 {
    if outgoing {
        WARM
    } else {
        ACCENT
    }
}

/// The cursor: a translucent wash of `color` ringed by its solid stroke.
pub fn highlight(painter: &egui::Painter, rect: egui::Rect, color: egui::Color32) {
    painter.rect(
        rect,
        ROW_RADIUS,
        color.linear_multiply(HIGHLIGHT_WASH),
        egui::Stroke::new(1.0, color),
        egui::StrokeKind::Inside,
    );
}

/// A progress bar: a hairline track with `frac` of it lit in `color`.
pub fn progress(painter: &egui::Painter, rect: egui::Rect, frac: f32, color: egui::Color32) {
    let radius = rect.height() / 2.0;
    painter.rect_filled(rect, radius, BORDER);
    let mut lit = rect;
    lit.set_width(rect.width() * frac.clamp(0.0, 1.0));
    if lit.width() >= rect.height() {
        painter.rect_filled(lit, radius, color);
    }
}

/// The centered floating card (incoming request, keyboard, toasts): one frame,
/// so they read as the same surface. `edge` rings it.
pub fn card_frame(edge: egui::Color32) -> egui::Frame {
    egui::Frame::new()
        .fill(PANEL_FILL)
        .stroke(egui::Stroke::new(1.0, edge))
        .corner_radius(10.0)
        .inner_margin(egui::Margin::same(14))
}

/// Component-wise sRGB lerp: fine for a brand tint, no need for linear space.
pub fn lerp_color(a: egui::Color32, b: egui::Color32, t: f32) -> egui::Color32 {
    let t = t.clamp(0.0, 1.0);
    let m = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round() as u8;
    egui::Color32::from_rgba_premultiplied(
        m(a.r(), b.r()),
        m(a.g(), b.g()),
        m(a.b(), b.b()),
        m(a.a(), b.a()),
    )
}

/// Install the palette on egui's dark theme: a translucent accent fill behind
/// selected widgets ringed by the solid accent, plus accent links and caret.
pub fn apply(ctx: &egui::Context) {
    // Hack holds glyphs the Proportional stack lacks (the arrow before a
    // save path); appended last, so ordinary text is unchanged.
    let mut fonts = egui::FontDefinitions::default();
    if let Some(prop) = fonts.families.get_mut(&egui::FontFamily::Proportional) {
        prop.push("Hack".to_owned());
    }
    ctx.set_fonts(fonts);

    let mut visuals = egui::Visuals::dark();
    visuals.selection.bg_fill = ACCENT.linear_multiply(HIGHLIGHT_WASH);
    visuals.selection.stroke = egui::Stroke::new(1.0, ACCENT);
    visuals.hyperlink_color = ACCENT;
    visuals.text_cursor.stroke.color = ACCENT;
    visuals.panel_fill = PANEL_FILL;
    visuals.override_text_color = Some(INK);
    ctx.set_visuals(visuals);
}
