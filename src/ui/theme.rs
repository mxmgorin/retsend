//! The egui visual theme: the brand palette, sizing constants for 640×480
//! gamepad-first screens, and the painters every screen shares. Same shape as
//! retsurf's theme so the two apps feel like siblings.
//!
//! The palette is the icon's: a striped sun (amber into coral) inside a teal
//! orbit. Teal marks what comes in and what is selected; coral marks what goes
//! out. Everything else is the dark plate the icon sits on.
//!
//! A [`Theme`] repaints the screens around the mark — neutrals, accents, the
//! cursor's shape and the font — keeping teal for in and coral for out in its
//! own hues; the mark and wordmark keep the brand colors in every theme. The
//! theme in force is process-wide: painters read it through [`palette`].

use crate::config::display::Theme;
use egui_sdl2::egui;
use std::sync::atomic::{AtomicUsize, Ordering};

mod palettes;

pub use palettes::{Cursor, Palette};

/// The orbit's teal.
pub const TEAL: egui::Color32 = egui::Color32::from_rgb(0x3f, 0xb8, 0xa0);
/// The sun's coral.
pub const CORAL: egui::Color32 = egui::Color32::from_rgb(0xff, 0x8c, 0x69);
/// The sun's amber top, and the orbit arrow's head.
pub const SUN_TOP: egui::Color32 = egui::Color32::from_rgb(0xff, 0xb8, 0x70);

/// Backdrop under a modal: what it covers reads as inactive.
pub const SCRIM: egui::Color32 = egui::Color32::from_black_alpha(160);

/// The theme in force, as its index in [`Theme::ALL`].
static ACTIVE: AtomicUsize = AtomicUsize::new(0);

/// The palette of the theme in force.
pub fn palette() -> &'static Palette {
    palettes::of(Theme::ALL[ACTIVE.load(Ordering::Relaxed)])
}

pub fn accent() -> egui::Color32 {
    palette().accent
}

pub fn warm() -> egui::Color32 {
    palette().warm
}

pub fn danger() -> egui::Color32 {
    palette().danger
}

pub fn bg() -> egui::Color32 {
    palette().bg
}

pub fn panel_fill() -> egui::Color32 {
    palette().panel_fill
}

pub fn surface() -> egui::Color32 {
    palette().surface
}

pub fn border() -> egui::Color32 {
    palette().border
}

pub fn ink() -> egui::Color32 {
    palette().ink
}

pub fn dim() -> egui::Color32 {
    palette().dim
}

/// [`bg`] as the GL clear color.
pub fn background() -> [f32; 4] {
    egui::Rgba::from(bg()).to_array()
}

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
/// The key a theme's font is registered under.
const THEME_FONT: &str = "theme";
/// Width of a [`Cursor::Bar`]'s leading stripe.
const BAR_STRIPE: f32 = 3.0;

/// The accent for what comes in, the warm color for what goes out.
pub fn flow_color(outgoing: bool) -> egui::Color32 {
    if outgoing {
        warm()
    } else {
        accent()
    }
}

/// The cursor over `rect`, marked in `color` the way the theme draws it.
pub fn highlight(painter: &egui::Painter, rect: egui::Rect, color: egui::Color32) {
    match palette().cursor {
        Cursor::Ring => {
            painter.rect(
                rect,
                ROW_RADIUS,
                color.linear_multiply(HIGHLIGHT_WASH),
                egui::Stroke::new(1.0, color),
                egui::StrokeKind::Inside,
            );
        }
        Cursor::Bar(fill) => {
            painter.rect_filled(rect, 0.0, fill);
            let stripe = egui::Rect::from_min_size(rect.min, egui::vec2(BAR_STRIPE, rect.height()));
            painter.rect_filled(stripe, 0.0, color);
        }
    }
}

/// A progress bar: a hairline track with `frac` of it lit in `color`.
pub fn progress(painter: &egui::Painter, rect: egui::Rect, frac: f32, color: egui::Color32) {
    let radius = rect.height() / 2.0;
    painter.rect_filled(rect, radius, border());
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
        .fill(panel_fill())
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

/// The font set for `palette`: its face ahead of egui's own, then Hack for
/// glyphs neither holds.
fn fonts(palette: &Palette) -> egui::FontDefinitions {
    let mut fonts = egui::FontDefinitions::default();
    if let Some(data) = palette.font {
        fonts.font_data.insert(
            THEME_FONT.to_owned(),
            egui::FontData::from_static(data).into(),
        );
    }
    if let Some(prop) = fonts.families.get_mut(&egui::FontFamily::Proportional) {
        if palette.font.is_some() {
            prop.insert(0, THEME_FONT.to_owned());
        }
        prop.push("Hack".to_owned());
    }
    fonts
}

/// Make `theme` the one in force and install its fonts and egui's dark visuals:
/// a translucent accent fill behind selected widgets ringed by the solid
/// accent, plus accent links and caret.
pub fn apply(ctx: &egui::Context, theme: Theme) {
    ACTIVE.store(theme.index(), Ordering::Relaxed);
    let palette = palette();
    ctx.set_fonts(fonts(palette));
    let mut visuals = egui::Visuals::dark();
    visuals.selection.bg_fill = palette.accent.linear_multiply(HIGHLIGHT_WASH);
    visuals.selection.stroke = egui::Stroke::new(1.0, palette.accent);
    visuals.hyperlink_color = palette.accent;
    visuals.text_cursor.stroke.color = palette.accent;
    visuals.panel_fill = palette.panel_fill;
    visuals.override_text_color = Some(palette.ink);
    ctx.set_visuals(visuals);
}
