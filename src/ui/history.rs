//! The History tab: the persisted transfer log, newest first. Rows show the
//! peer + direction, a counts/size/relative-time detail line, and an outcome
//! glyph. Read-only; the cursor just scrolls.

use super::{fmt_bytes, theme};
use crate::transfer::history::{Direction, HistoryEntry, Outcome};
use egui_sdl2::egui;

/// A display-ready history row, built by `AppUi::update` from a [`HistoryEntry`].
pub struct HistoryRow {
    pub title: String,
    pub detail: String,
    pub outcome: Outcome,
}

pub struct HistoryData {
    pub rows: Vec<HistoryRow>,
    pub cursor: Option<usize>,
}

/// Build a row from an entry, resolving the relative time against `now`
/// (unix seconds).
pub fn row(e: &HistoryEntry, now: u64) -> HistoryRow {
    let (verb, prep) = match e.direction {
        Direction::Received => ("Received", "from"),
        Direction::Sent => ("Sent", "to"),
    };
    let what = match e.outcome {
        Outcome::Completed => plural(e.total),
        Outcome::Partial => format!("{}/{} files", e.done, e.total),
        Outcome::Cancelled => "cancelled".to_string(),
        Outcome::Declined => "declined".to_string(),
        Outcome::Failed => "failed".to_string(),
    };
    HistoryRow {
        title: format!("{verb} {prep} {}", e.peer),
        detail: format!("{what} · {} · {}", fmt_bytes(e.bytes), ago(now, e.at)),
        outcome: e.outcome,
    }
}

pub fn render(root: &mut egui::Ui, data: &HistoryData) {
    egui::Panel::bottom("tab_footer").show(root, |ui| {
        ui.add_space(4.0);
        super::home::hint_bar(ui, &[("L1/R1", "Tabs")]);
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

        egui::ScrollArea::vertical().show(ui, |ui| {
            for (i, row) in data.rows.iter().enumerate() {
                let response = history_row(ui, row, data.cursor == Some(i));
                if data.cursor == Some(i) {
                    response.scroll_to_me(None);
                }
            }
        });
    });
}

fn history_row(ui: &mut egui::Ui, row: &HistoryRow, selected: bool) -> egui::Response {
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
    let (glyph, glyph_color) = glyph(row.outcome);
    let painter = ui.painter();
    painter.text(
        rect.left_top() + egui::vec2(padding, 7.0),
        egui::Align2::LEFT_TOP,
        &row.title,
        egui::FontId::proportional(theme::ROW_FONT),
        ui.visuals().text_color(),
    );
    painter.text(
        rect.left_bottom() + egui::vec2(padding, -7.0),
        egui::Align2::LEFT_BOTTOM,
        &row.detail,
        egui::FontId::proportional(theme::DETAIL_FONT),
        theme::DIM,
    );
    painter.text(
        rect.right_center() - egui::vec2(padding, 0.0),
        egui::Align2::RIGHT_CENTER,
        glyph,
        egui::FontId::proportional(theme::ROW_FONT),
        glyph_color,
    );
    response
}

/// Outcome → (glyph, color): a green check when everything landed, a dim check
/// for a partial, a red cross for a failure/decline.
fn glyph(outcome: Outcome) -> (&'static str, egui::Color32) {
    match outcome {
        Outcome::Completed => ("✓", theme::ACCENT),
        Outcome::Partial => ("✓", theme::DIM),
        Outcome::Cancelled => ("✗", theme::DIM),
        Outcome::Declined | Outcome::Failed => ("✗", theme::DANGER),
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
