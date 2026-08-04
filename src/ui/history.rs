//! The History tab: the persisted transfer log, newest first. Rows show the
//! peer + direction, a counts/size/relative-time detail line, the directory the
//! files landed in (or came from), and an outcome glyph. Read-only; the cursor
//! just scrolls.

use super::{fmt_bytes, theme, truncate_middle, PATH_CHARS};
use crate::transfer::history::{Direction, HistoryEntry, Outcome};
use egui_sdl2::egui;

/// Height the path line adds to a row that has one.
const PATH_LINE: f32 = 16.0;

/// A display-ready history row, built by `AppUi::update` from a [`HistoryEntry`].
pub struct HistoryRow {
    pub title: String,
    pub detail: String,
    /// Arrow-prefixed save/source directory; empty when the entry has none.
    pub path: String,
    pub outcome: Outcome,
}

pub struct HistoryData {
    pub rows: Vec<HistoryRow>,
    pub cursor: Option<usize>,
}

/// Build a row from an entry, resolving the relative time against `now`
/// (unix seconds).
pub fn row(e: &HistoryEntry, now: u64) -> HistoryRow {
    let (verb, prep, arrow) = match e.direction {
        Direction::Received => ("Received", "from", "→"),
        Direction::Sent => ("Sent", "to", "←"),
    };
    let what = match e.outcome {
        Outcome::Completed => plural(e.total),
        Outcome::Partial => format!("{}/{} files", e.done, e.total),
        Outcome::Cancelled => "cancelled".to_string(),
        Outcome::Declined => "declined".to_string(),
        Outcome::Failed => "failed".to_string(),
    };
    let path = if e.path.is_empty() {
        String::new()
    } else {
        format!("{arrow} {}", truncate_middle(&e.path, PATH_CHARS))
    };
    HistoryRow {
        title: format!("{verb} {prep} {}", e.peer),
        detail: format!("{what} · {} · {}", fmt_bytes(e.bytes), ago(now, e.at)),
        path,
        outcome: e.outcome,
    }
}

pub fn render(root: &mut egui::Ui, data: &HistoryData) {
    egui::Panel::bottom(super::BOTTOM_PANEL_ID).show(root, |ui| {
        ui.add_space(4.0);
        super::home::hint_bar(ui, &[("← →", "Tabs")]);
        ui.add_space(4.0);
    });

    egui::CentralPanel::default().show(root, |ui| {
        if data.rows.is_empty() {
            let top = (ui.available_height() / 2.0 - 20.0).max(8.0);
            ui.vertical_centered(|ui| {
                ui.add_space(top);
                ui.label(
                    egui::RichText::new("No transfers yet.")
                        .size(theme::ROW_FONT)
                        .color(theme::DIM),
                );
            });
            return;
        }

        // Virtualized like the browser; rows differ in height (path line or
        // not), so tops are prefix sums rather than a fixed step.
        let spacing = ui.spacing().item_spacing.y;
        let mut tops = Vec::with_capacity(data.rows.len() + 1);
        let mut y = 0.0;
        for row in &data.rows {
            tops.push(y);
            y += row_height(row) + spacing;
        }
        tops.push(y);
        egui::ScrollArea::vertical().show_viewport(ui, |ui, viewport| {
            ui.set_height(y - spacing);
            ui.set_width(ui.available_width());
            let origin = ui.min_rect().left_top();
            let width = ui.available_width();
            let row_rect = |i: usize| {
                egui::Rect::from_min_size(
                    origin + egui::vec2(0.0, tops[i]),
                    egui::vec2(width, row_height(&data.rows[i])),
                )
            };
            if let Some(c) = data.cursor.filter(|&c| c < data.rows.len()) {
                ui.scroll_to_rect(row_rect(c), None);
            }
            let first = tops
                .partition_point(|&t| t <= viewport.min.y)
                .saturating_sub(1);
            for (i, row) in data.rows.iter().enumerate().skip(first) {
                if tops[i] > viewport.max.y {
                    break;
                }
                history_row(ui, row, row_rect(i), data.cursor == Some(i));
            }
        });
    });
}

fn row_height(row: &HistoryRow) -> f32 {
    theme::ROW_HEIGHT + if row.path.is_empty() { 0.0 } else { PATH_LINE }
}

fn history_row(ui: &mut egui::Ui, row: &HistoryRow, rect: egui::Rect, selected: bool) {
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
    let (glyph, glyph_color) = glyph(row.outcome);
    let painter = ui.painter();
    painter.text(
        rect.left_top() + egui::vec2(padding, 7.0),
        egui::Align2::LEFT_TOP,
        &row.title,
        egui::FontId::proportional(theme::ROW_FONT),
        ui.visuals().text_color(),
    );
    // Title and detail keep their places in the first ROW_HEIGHT; the path line
    // hangs below it.
    painter.text(
        rect.left_top() + egui::vec2(padding, theme::ROW_HEIGHT - 7.0),
        egui::Align2::LEFT_BOTTOM,
        &row.detail,
        egui::FontId::proportional(theme::DETAIL_FONT),
        theme::DIM,
    );
    if !row.path.is_empty() {
        painter.text(
            rect.left_bottom() + egui::vec2(padding, -4.0),
            egui::Align2::LEFT_BOTTOM,
            &row.path,
            egui::FontId::proportional(theme::DETAIL_FONT),
            theme::DIM,
        );
    }
    painter.text(
        egui::pos2(rect.right() - padding, rect.top() + theme::ROW_HEIGHT / 2.0),
        egui::Align2::RIGHT_CENTER,
        glyph,
        egui::FontId::proportional(theme::ROW_FONT),
        glyph_color,
    );
}

/// Outcome → (glyph, color): a green check when everything landed, a dim check
/// for a partial, a red cross for a failure/decline.
fn glyph(outcome: Outcome) -> (&'static str, egui::Color32) {
    match outcome {
        Outcome::Completed => ("√", theme::ACCENT),
        Outcome::Partial => ("√", theme::DIM),
        Outcome::Cancelled => ("×", theme::DIM),
        Outcome::Declined | Outcome::Failed => ("×", theme::DANGER),
    }
}

fn plural(n: usize) -> String {
    if n == 1 {
        "1 file".to_string()
    } else {
        format!("{n} files")
    }
}

/// Compact relative time: "just now", "5m ago", "3h ago", "2d ago".
fn ago(now: u64, at: u64) -> String {
    let s = now.saturating_sub(at);
    if s < 60 {
        "just now".to_string()
    } else if s < 3600 {
        format!("{}m ago", s / 60)
    } else if s < 86400 {
        format!("{}h ago", s / 3600)
    } else {
        format!("{}d ago", s / 86400)
    }
}
