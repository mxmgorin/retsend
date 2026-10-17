//! One [`Palette`] per [`Theme`]. A new theme is a variant of [`Theme`] and a
//! palette here, its font prepared by `tools/theme-font.sh`.

use crate::config::display::Theme;
use egui_sdl2::egui;

/// The palette `theme` paints with.
pub fn of(theme: Theme) -> &'static Palette {
    match theme {
        Theme::Retsend => &RETSEND,
        Theme::Spruce => &SPRUCE,
    }
}

/// The colors, cursor and font a theme paints the screens with.
pub struct Palette {
    /// Incoming transfers, selection, links.
    pub accent: egui::Color32,
    /// Outgoing transfers.
    pub warm: egui::Color32,
    /// Failure red: declined, failed, no network, seconds running out.
    pub danger: egui::Color32,
    /// The plate the icon sits on; the window clear color.
    pub bg: egui::Color32,
    /// Header and footer bars, cards, keys: one step up from the plate.
    pub panel_fill: egui::Color32,
    /// Raised controls: keys, a switch that is off.
    pub surface: egui::Color32,
    /// Hairlines: card edges, progress tracks, section rules.
    pub border: egui::Color32,
    /// Primary text, matching the wordmark.
    pub ink: egui::Color32,
    /// Secondary text: hints, device details, statuses.
    pub dim: egui::Color32,
    pub cursor: Cursor,
    /// A face put ahead of egui's own for proportional text.
    pub font: Option<&'static [u8]>,
    /// Who the theme's look and font are owed to.
    pub credit: Option<&'static str>,
}

/// How the cursor marks the row under it.
pub enum Cursor {
    /// A translucent wash of the row's color ringed by its solid stroke.
    Ring,
    /// A solid bar in this fill, the row's color as a stripe on its leading edge.
    Bar(egui::Color32),
}

/// The icon's own dark plate.
pub const RETSEND: Palette = Palette {
    accent: super::TEAL,
    warm: super::CORAL,
    danger: egui::Color32::from_rgb(0xe0, 0x5a, 0x5a),
    bg: egui::Color32::from_rgb(0x16, 0x17, 0x1a),
    panel_fill: egui::Color32::from_rgb(0x1b, 0x1c, 0x20),
    surface: egui::Color32::from_rgb(0x24, 0x26, 0x2b),
    border: egui::Color32::from_rgb(0x2e, 0x31, 0x37),
    ink: egui::Color32::from_rgb(0xec, 0xec, 0xea),
    dim: egui::Color32::from_rgb(0x8f, 0x94, 0x9c),
    cursor: Cursor::Ring,
    font: None,
    credit: None,
};

/// spruceOS's stock theme: gruvbox colors, a solid list bar, its Nunito.
pub const SPRUCE: Palette = Palette {
    accent: egui::Color32::from_rgb(0x8e, 0xc0, 0x7c),
    warm: egui::Color32::from_rgb(0xfe, 0x80, 0x19),
    danger: egui::Color32::from_rgb(0xfb, 0x49, 0x34),
    bg: egui::Color32::from_rgb(0x28, 0x28, 0x28),
    panel_fill: egui::Color32::from_rgb(0x32, 0x30, 0x2f),
    surface: egui::Color32::from_rgb(0x3c, 0x38, 0x36),
    border: egui::Color32::from_rgb(0x50, 0x49, 0x45),
    ink: egui::Color32::from_rgb(0xeb, 0xdb, 0xb2),
    dim: egui::Color32::from_rgb(0xa8, 0x99, 0x84),
    cursor: Cursor::Bar(egui::Color32::from_rgb(0x50, 0x49, 0x45)),
    font: Some(include_bytes!("../../../resources/fonts/nunwen.ttf")),
    credit: Some("Theme inspired by spruceOS, by tenlevels · Nunito, SIL OFL 1.1"),
};
