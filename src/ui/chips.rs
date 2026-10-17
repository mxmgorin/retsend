//! Chips: an icon and a line of text in a pill, for the facts a landing
//! screen keeps along its bottom edge. Packed into centered rows no wider than
//! the screen, so a narrow panel wraps them instead of cutting them.

use super::icons::{self, Icon};
use super::theme;
use egui_sdl2::egui;

/// An icon, a line of text, and the color the icon is drawn in. Text is dim
/// unless the icon carries a meaning of its own (a warning, the accent).
pub struct Chip {
    pub icon: Icon,
    pub text: String,
    pub color: egui::Color32,
}

/// Chip metrics: height, inner padding, icon-to-text gap, and the gaps
/// between chips and between rows.
const CHIP_H: f32 = 26.0;
const CHIP_PAD: f32 = 10.0;
const CHIP_ICON_GAP: f32 = 6.0;
const CHIP_GAP: f32 = 8.0;
const ROW_GAP: f32 = 6.0;
const CHIP_ICON: f32 = theme::ICON_SIZE - 1.0;
/// Space the chips keep from the panel's bottom edge.
const BOTTOM: f32 = 10.0;

/// Chips packed into rows of at most `width`, keeping their order.
pub struct Rows<'a>(Vec<Vec<(&'a Chip, f32)>>);

impl<'a> Rows<'a> {
    pub fn pack(ui: &egui::Ui, chips: &'a [Chip], width: f32) -> Self {
        let mut rows: Vec<Vec<(&Chip, f32)>> = vec![Vec::new()];
        let mut used = 0.0;
        for chip in chips {
            let w = chip_width(ui, chip);
            let row = rows.last_mut().expect("pack starts with one row");
            let needed = if row.is_empty() {
                w
            } else {
                used + CHIP_GAP + w
            };
            if !row.is_empty() && needed > width {
                rows.push(vec![(chip, w)]);
                used = w;
            } else {
                row.push((chip, w));
                used = needed;
            }
        }
        Rows(rows)
    }

    /// Height the rows take at the bottom of a panel, their margin included.
    pub fn height(&self) -> f32 {
        if self.0.iter().all(Vec::is_empty) {
            return 0.0;
        }
        let n = self.0.len() as f32;
        n * CHIP_H + (n - 1.0) * ROW_GAP + BOTTOM
    }

    /// Paint the rows along the bottom of `ui`'s rect, each row centered.
    pub fn paint(&self, ui: &egui::Ui) {
        let panel = ui.max_rect();
        let mut y = panel.bottom() - self.height();
        for row in &self.0 {
            let width: f32 =
                row.iter().map(|(_, w)| *w).sum::<f32>() + CHIP_GAP * (row.len() as f32 - 1.0);
            let mut x = panel.center().x - width / 2.0;
            for (chip, w) in row {
                let rect = egui::Rect::from_min_size(egui::pos2(x, y), egui::vec2(*w, CHIP_H));
                paint_chip(ui, chip, rect);
                x += w + CHIP_GAP;
            }
            y += CHIP_H + ROW_GAP;
        }
    }
}

fn chip_width(ui: &egui::Ui, chip: &Chip) -> f32 {
    let galley = ui.painter().layout_no_wrap(
        chip.text.clone(),
        egui::FontId::proportional(theme::DETAIL_FONT),
        theme::dim(),
    );
    CHIP_PAD + CHIP_ICON + CHIP_ICON_GAP + galley.size().x + CHIP_PAD
}

fn paint_chip(ui: &egui::Ui, chip: &Chip, rect: egui::Rect) {
    let painter = ui.painter();
    painter.rect(
        rect,
        CHIP_H / 2.0,
        theme::panel_fill(),
        egui::Stroke::new(1.0, theme::border()),
        egui::StrokeKind::Inside,
    );
    let icon_center = egui::pos2(rect.left() + CHIP_PAD + CHIP_ICON / 2.0, rect.center().y);
    icons::paint(painter, chip.icon, icon_center, CHIP_ICON, chip.color);
    let text_color = if chip.color == theme::dim() {
        theme::dim()
    } else {
        chip.color
    };
    painter.text(
        egui::pos2(
            rect.left() + CHIP_PAD + CHIP_ICON + CHIP_ICON_GAP,
            rect.center().y,
        ),
        egui::Align2::LEFT_CENTER,
        &chip.text,
        egui::FontId::proportional(theme::DETAIL_FONT),
        text_color,
    );
}

/// The device-name line under a landing screen's mark.
pub const NAME_FONT: f32 = 22.0;

/// The mark, a big line and a dim line under it, centered in the room the
/// chips leave. `spin` and `dim` go to [`super::mark::paint`].
pub fn hero(
    ui: &mut egui::Ui,
    chips_h: f32,
    spin: f32,
    dim: f32,
    name: &str,
    name_color: egui::Color32,
    status: &str,
) {
    // The mark, the gap under it, and the two lines, roughly.
    const MARK_GAP: f32 = 18.0;
    const LINES_H: f32 = 50.0;
    let hero_h = super::mark::HERO_SIZE + MARK_GAP + LINES_H;
    let free = ui.available_height() - chips_h;
    let top = ((free - hero_h) / 2.0).max(8.0);
    ui.vertical_centered(|ui| {
        ui.add_space(top);
        let (_, rect) = ui.allocate_space(super::mark::measure(super::mark::HERO_SIZE));
        super::mark::paint(
            ui.painter(),
            rect.center(),
            super::mark::HERO_SIZE,
            spin,
            dim,
        );
        ui.add_space(MARK_GAP);
        ui.label(
            egui::RichText::new(name)
                .size(NAME_FONT)
                .strong()
                .color(name_color),
        );
        ui.add_space(2.0);
        ui.label(
            egui::RichText::new(status)
                .size(theme::DETAIL_FONT)
                .color(theme::dim()),
        );
    });
}
