//! The About screen: the brand mark and wordmark over a small block of build
//! metadata (version, build date, commit), the project URL and author.
//! Credits close it: what retsend is built on, then the theme's own if it has
//! one.
//! Read-only — B backs out to Settings. All values are compile-time constants
//! (`CARGO_PKG_*` plus the `RETSEND_*` vars stamped by `build.rs`), so nothing
//! is threaded in.

use super::{mark, theme};
use crate::app::AppCommand;
use egui_sdl2::egui;

/// The wordmark's type size under the hero.
const WORDMARK_SIZE: f32 = 30.0;
/// Gaps down the block: under the mark, above the table, above the credits.
const MARK_GAP: f32 = 8.0;
const TABLE_GAP: f32 = 20.0;
const CREDITS_GAP: f32 = 16.0;
/// Between a label and its value, and between consecutive lines.
const COLUMN_GAP: f32 = 12.0;
const LINE_GAP: f32 = 4.0;
/// Least room above the block when it fills the screen.
const MIN_TOP: f32 = 8.0;

/// Build metadata, as label and value.
const INFO: [(&str, &str); 5] = [
    ("Version", env!("CARGO_PKG_VERSION")),
    ("Built", env!("RETSEND_BUILD_DATE")),
    ("Commit", env!("RETSEND_GIT_COMMIT")),
    ("Project", env!("CARGO_PKG_REPOSITORY")),
    ("Author", env!("CARGO_PKG_AUTHORS")),
];
/// Dropped from a link: the screen shows it to be read, not followed.
const SCHEME: &str = "https://";
/// What retsend is built on; the theme's own credit follows it.
const CREDITS: &str = "LocalSend protocol · SDL2 · egui";

pub fn render(root: &mut egui::Ui, taps: &mut Vec<AppCommand>) {
    // A decorative header: every base screen draws one top and one bottom
    // panel under the shared ids, see [`super::TOP_PANEL_ID`].
    egui::Panel::top(super::TOP_PANEL_ID).show(root, |ui| {
        ui.add_space(6.0);
        ui.label(
            egui::RichText::new("About")
                .size(theme::TITLE_FONT)
                .strong(),
        );
        ui.add_space(6.0);
    });

    egui::Panel::bottom(super::BOTTOM_PANEL_ID).show(root, |ui| {
        ui.add_space(4.0);
        super::home::hint_bar(ui, &[("B", "Back", Some(AppCommand::Back))], taps);
        ui.add_space(4.0);
    });

    egui::CentralPanel::default().show(root, |ui| {
        let painter = ui.painter();
        let font = egui::FontId::proportional(theme::DETAIL_FONT);
        let line = |text: &str, color| painter.layout_no_wrap(text.into(), font.clone(), color);
        let wordmark = mark::wordmark(ui, WORDMARK_SIZE);
        let description = line(env!("CARGO_PKG_DESCRIPTION"), theme::dim());
        let rows: Vec<_> = INFO
            .iter()
            .map(|(label, value)| {
                let value = value.strip_prefix(SCHEME).unwrap_or(value);
                (line(label, theme::dim()), line(value, theme::accent()))
            })
            .collect();
        let credits: Vec<_> = std::iter::once(CREDITS)
            .chain(theme::palette().credit)
            .map(|text| line(text, theme::dim()))
            .collect();

        let table_h = stack_height(rows.iter().map(|(label, _)| label));
        let credits_h = stack_height(credits.iter());
        let total = mark::HERO_SIZE
            + MARK_GAP
            + wordmark.size().y
            + description.size().y
            + TABLE_GAP
            + table_h
            + CREDITS_GAP
            + credits_h;
        let area = ui.max_rect();
        let center_x = area.center().x;
        let mut y = area.top() + ((area.height() - total) / 2.0).max(MIN_TOP);

        mark::paint(
            painter,
            egui::pos2(center_x, y + mark::HERO_SIZE / 2.0),
            mark::HERO_SIZE,
            0.0,
            0.0,
        );
        y += mark::HERO_SIZE + MARK_GAP;
        y += centered(painter, center_x, y, wordmark, theme::ink());
        y += centered(painter, center_x, y, description, theme::dim()) + TABLE_GAP;

        // Labels flush right and values flush left of one axis, the pair
        // centered as a whole.
        let label_w = rows.iter().map(|(l, _)| l.size().x).fold(0.0, f32::max);
        let value_w = rows.iter().map(|(_, v)| v.size().x).fold(0.0, f32::max);
        let axis = center_x - (label_w + COLUMN_GAP + value_w) / 2.0 + label_w;
        for (label, value) in rows {
            let h = label.size().y;
            painter.galley(egui::pos2(axis - label.size().x, y), label, theme::dim());
            painter.galley(egui::pos2(axis + COLUMN_GAP, y), value, theme::accent());
            y += h + LINE_GAP;
        }

        y += CREDITS_GAP - LINE_GAP;
        for credit in credits {
            y += centered(painter, center_x, y, credit, theme::dim()) + LINE_GAP;
        }
    });
}

/// Paints `galley` centered on `center_x` with its top at `y`; returns its height.
fn centered(
    painter: &egui::Painter,
    center_x: f32,
    y: f32,
    galley: std::sync::Arc<egui::Galley>,
    color: egui::Color32,
) -> f32 {
    let size = galley.size();
    painter.galley(egui::pos2(center_x - size.x / 2.0, y), galley, color);
    size.y
}

/// Height of `lines` stacked [`LINE_GAP`] apart.
fn stack_height<'a>(lines: impl Iterator<Item = &'a std::sync::Arc<egui::Galley>>) -> f32 {
    let (sum, count) = lines.fold((0.0, 0_usize), |(sum, n), g| (sum + g.size().y, n + 1));
    sum + LINE_GAP * count.saturating_sub(1) as f32
}
