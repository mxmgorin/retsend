//! The History tab: the persisted transfer log, newest first. Rows show the
//! direction, the peer, a counts/size/relative-time detail line, the directory
//! the files landed in (or came from), and an outcome mark. A row can be
//! dropped from the log (X), and a send can be repeated from it (A).

use super::home::Hint;
use super::icons::{self, Icon};
use super::{fmt_bytes, mark, theme, truncate_middle, PATH_CHARS};
use crate::app::AppCommand;
use crate::transfer::history::{Direction, HistoryEntry, Outcome};
use egui_sdl2::egui;

/// Height the path line adds to a row that has one.
const PATH_LINE: f32 = 16.0;
/// Gutter the direction icon takes before the text.
const ICON_GUTTER: f32 = 30.0;
/// How far the hero behind an empty log is pulled toward the plate.
const EMPTY_DIM: f32 = 0.55;

/// A display-ready history row; [`row`] builds one from a [`HistoryEntry`].
pub struct HistoryRow {
    pub title: String,
    pub detail: String,
    /// Save/source directory; empty when the entry has none.
    pub path: String,
    pub outgoing: bool,
    pub outcome: Outcome,
}

pub struct HistoryData {
    pub rows: Vec<HistoryRow>,
    pub cursor: Option<usize>,
    /// The row under the cursor is a send whose files are still recorded, so
    /// the resend hint is offered.
    pub can_resend: bool,
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
        path: truncate_middle(&e.path, PATH_CHARS),
        outgoing: e.direction == Direction::Sent,
        outcome: e.outcome,
    }
}

pub fn render(root: &mut egui::Ui, data: &HistoryData, taps: &mut Vec<AppCommand>) {
    egui::Panel::bottom(super::BOTTOM_PANEL_ID).show(root, |ui| {
        ui.add_space(4.0);
        let mut hints: Vec<Hint> = vec![("← →", "Tabs", None)];
        if !data.rows.is_empty() {
            hints.push(("X", "Delete", Some(AppCommand::Alt)));
        }
        if data.can_resend {
            hints.push(("A", "Send again", Some(AppCommand::Confirm)));
        }
        if crate::app::back_quits() {
            hints.push(("B", "Quit", Some(AppCommand::Back)));
        }
        super::home::hint_bar(ui, &hints, taps);
        ui.add_space(4.0);
    });

    egui::CentralPanel::default().show(root, |ui| {
        if data.rows.is_empty() {
            const HERO_H: f32 = mark::HERO_SIZE + 64.0; // mark + gap + two lines, roughly
            let top = ((ui.available_height() - HERO_H) / 2.0).max(8.0);
            ui.vertical_centered(|ui| {
                ui.add_space(top);
                let (_, rect) = ui.allocate_space(mark::measure(mark::HERO_SIZE));
                mark::paint(ui.painter(), rect.center(), mark::HERO_SIZE, 0.0, EMPTY_DIM);
                ui.add_space(20.0);
                ui.label(
                    egui::RichText::new("No transfers yet.")
                        .size(theme::ROW_FONT)
                        .color(theme::DIM),
                );
                ui.add_space(6.0);
                ui.label(
                    egui::RichText::new("Files sent and received show up here.")
                        .size(theme::DETAIL_FONT)
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
        egui::ScrollArea::vertical()
            .id_salt("history")
            .show_viewport(ui, |ui, viewport| {
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
                    super::scroll_to_cursor(ui, ("history", c), row_rect(c));
                }
                let first = tops
                    .partition_point(|&t| t <= viewport.min.y)
                    .saturating_sub(1);
                // A tap only carries the cursor; deleting and resending stay on
                // their hints, so neither fires from a mis-tap on a row.
                let tap = super::home::tap_pos(ui);
                for (i, row) in data.rows.iter().enumerate().skip(first) {
                    if tops[i] > viewport.max.y {
                        break;
                    }
                    let rect = row_rect(i);
                    if tap.is_some_and(|pos| rect.contains(pos)) {
                        taps.push(AppCommand::PickRow(i));
                    }
                    history_row(ui, row, rect, data.cursor == Some(i));
                }
            });
    });
}

fn row_height(row: &HistoryRow) -> f32 {
    theme::ROW_HEIGHT + if row.path.is_empty() { 0.0 } else { PATH_LINE }
}

fn history_row(ui: &mut egui::Ui, row: &HistoryRow, rect: egui::Rect, selected: bool) {
    let flow = theme::flow_color(row.outgoing);
    if selected {
        theme::highlight(ui.painter(), rect, flow);
    }
    let painter = ui.painter();
    let direction = if row.outgoing {
        Icon::Send
    } else {
        Icon::Receive
    };
    icons::paint(
        painter,
        direction,
        egui::pos2(
            rect.left() + theme::ROW_PAD + theme::ICON_SIZE / 2.0,
            rect.top() + theme::ROW_HEIGHT / 2.0,
        ),
        theme::ICON_SIZE + 3.0,
        flow,
    );
    let text_x = theme::ROW_PAD + ICON_GUTTER;
    painter.text(
        rect.left_top() + egui::vec2(text_x, 7.0),
        egui::Align2::LEFT_TOP,
        &row.title,
        egui::FontId::proportional(theme::ROW_FONT),
        theme::INK,
    );
    // Title and detail keep their places in the first ROW_HEIGHT; the path line
    // hangs below it.
    painter.text(
        rect.left_top() + egui::vec2(text_x, theme::ROW_HEIGHT - 7.0),
        egui::Align2::LEFT_BOTTOM,
        &row.detail,
        egui::FontId::proportional(theme::DETAIL_FONT),
        theme::DIM,
    );
    if !row.path.is_empty() {
        painter.text(
            rect.left_bottom() + egui::vec2(text_x, -4.0),
            egui::Align2::LEFT_BOTTOM,
            &row.path,
            egui::FontId::proportional(theme::DETAIL_FONT),
            theme::DIM,
        );
    }
    let (icon, color) = outcome_mark(row.outcome, flow);
    icons::paint(
        painter,
        icon,
        egui::pos2(
            rect.right() - theme::ROW_PAD - theme::ICON_SIZE / 2.0,
            rect.top() + theme::ROW_HEIGHT / 2.0,
        ),
        theme::ICON_SIZE,
        color,
    );
}

/// Outcome → (icon, color): a check in the flow's color when everything
/// landed, a dim check for a partial, a red cross for a failure/decline.
fn outcome_mark(outcome: Outcome, flow: egui::Color32) -> (Icon, egui::Color32) {
    match outcome {
        Outcome::Completed => (Icon::Check, flow),
        Outcome::Partial => (Icon::Check, theme::DIM),
        Outcome::Cancelled => (Icon::Cross, theme::DIM),
        Outcome::Declined | Outcome::Failed => (Icon::Cross, theme::DANGER),
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
