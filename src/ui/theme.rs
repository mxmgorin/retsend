//! The egui visual theme, shared accent, and sizing constants for 640×480
//! gamepad-first screens. Same shape as retsurf's theme so the two apps feel
//! like siblings.

use egui_sdl2::egui;

/// The brand accent (teal-green) — selected/active emphasis across the UI.
pub const ACCENT: egui::Color32 = egui::Color32::from_rgb(0x3f, 0xb8, 0xa0);

/// Dark chrome panel fill (header/footer bars, settings).
pub const PANEL_FILL: egui::Color32 = egui::Color32::from_rgb(0x18, 0x18, 0x1c);

/// Window clear color behind the central list.
pub const BACKGROUND: [f32; 4] = [0.06, 0.06, 0.07, 1.0];

/// Secondary / label text: hints, device details, statuses.
pub const DIM: egui::Color32 = egui::Color32::from_gray(0x99);

/// Primary row font size (logical px) — readable on a 3.5" 640×480 panel.
pub const ROW_FONT: f32 = 16.0;
/// Secondary line under a row title.
pub const DETAIL_FONT: f32 = 12.5;
/// Minimum height of a navigable row.
pub const ROW_HEIGHT: f32 = 44.0;

/// Install the accent on egui's dark theme: a translucent accent fill behind
/// selected widgets ringed by the solid accent, plus accent links and caret.
pub fn apply(ctx: &egui::Context) {
    let mut visuals = egui::Visuals::dark();
    visuals.selection.bg_fill = ACCENT.linear_multiply(0.30);
    visuals.selection.stroke = egui::Stroke::new(1.0, ACCENT);
    visuals.hyperlink_color = ACCENT;
    visuals.text_cursor.stroke.color = ACCENT;
    visuals.panel_fill = PANEL_FILL;
    ctx.set_visuals(visuals);
}
