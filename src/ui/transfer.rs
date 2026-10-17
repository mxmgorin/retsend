//! The transfer screen: overall progress with speed/ETA, per-file rows with a
//! bar of their own, the finish summary, and the two-step cancel confirmation.
//! Colored by direction: teal coming in, coral going out.

use super::icons::{self, Icon};
use super::theme;
use crate::app::AppCommand;
use egui_sdl2::egui;
use std::time::Duration;

pub struct TransferData {
    /// The screen title, from the session's direction and phase.
    pub title: String,
    pub outgoing: bool,
    pub finished: bool,
    pub transferred: u64,
    pub total: u64,
    pub speed_bps: Option<f64>,
    /// From the first byte moved until the transfer finished.
    pub elapsed: Duration,
    pub rows: Vec<FileRow>,
    pub confirm_cancel: bool,
}

pub struct FileRow {
    pub name: String,
    pub size: u64,
    pub state: RowState,
    /// 0.0..=1.0 of this file.
    pub frac: f32,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum RowState {
    Pending,
    Moving,
    Done,
    Failed,
}

/// Height of the overall bar, and of a file's own.
const BAR_H: f32 = 8.0;
const FILE_BAR_H: f32 = 3.0;
/// A file row: name and size on one line, its bar under them.
const FILE_ROW_H: f32 = 28.0;
/// Gutter the state mark takes before a file's name.
const MARK_GUTTER: f32 = 26.0;
/// The summary's mark, bigger than a row's.
const SUMMARY_ICON: f32 = 30.0;

pub fn render(root: &mut egui::Ui, data: &TransferData, taps: &mut Vec<AppCommand>) {
    let flow = theme::flow_color(data.outgoing);
    egui::Panel::top(super::TOP_PANEL_ID).show(root, |ui| {
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            let (rect, _) = ui.allocate_exact_size(
                egui::Vec2::splat(theme::ICON_SIZE + 4.0),
                egui::Sense::hover(),
            );
            let icon = if data.outgoing {
                Icon::Send
            } else {
                Icon::Receive
            };
            icons::paint(
                ui.painter(),
                icon,
                rect.center(),
                theme::ICON_SIZE + 3.0,
                flow,
            );
            ui.label(
                egui::RichText::new(&data.title)
                    .size(theme::TITLE_FONT)
                    .strong(),
            );
        });
        ui.add_space(6.0);
    });

    egui::Panel::bottom(super::BOTTOM_PANEL_ID).show(root, |ui| {
        ui.add_space(4.0);
        if data.confirm_cancel {
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new("Cancel the transfer?")
                        .size(theme::DETAIL_FONT)
                        .strong(),
                );
                super::home::hint_bar(
                    ui,
                    &[
                        ("B", "Keep going", Some(AppCommand::Back)),
                        ("A", "Yes, cancel", Some(AppCommand::Confirm)),
                    ],
                    taps,
                );
            });
        } else if data.finished {
            super::home::hint_bar(ui, &[("B", "Back", Some(AppCommand::Back))], taps);
        } else {
            super::home::hint_bar(ui, &[("B", "Cancel", Some(AppCommand::Back))], taps);
        }
        ui.add_space(4.0);
    });

    egui::CentralPanel::default().show(root, |ui| {
        let frac = if data.total > 0 {
            data.transferred as f32 / data.total as f32
        } else {
            1.0
        };
        if data.finished {
            summary(ui, data, flow);
        } else {
            let (rect, _) = ui.allocate_exact_size(
                egui::vec2(ui.available_width(), BAR_H),
                egui::Sense::hover(),
            );
            theme::progress(ui.painter(), rect, frac, flow);
            let mut line = format!(
                "{} / {}",
                super::fmt_bytes(data.transferred),
                super::fmt_bytes(data.total)
            );
            if let Some(bps) = data.speed_bps {
                line.push_str(&format!(" · {}/s", super::fmt_bytes(bps as u64)));
                let remaining = data.total.saturating_sub(data.transferred);
                if bps > 1.0 && remaining > 0 {
                    line.push_str(&format!(
                        " · ~{}",
                        fmt_eta((remaining as f64 / bps).ceil() as u64)
                    ));
                }
            }
            ui.add_space(4.0);
            ui.label(
                egui::RichText::new(line)
                    .size(theme::DETAIL_FONT)
                    .color(theme::dim()),
            );
        }
        ui.add_space(10.0);

        egui::ScrollArea::vertical()
            .id_salt("transfer")
            .show(ui, |ui| {
                for row in &data.rows {
                    file_row(ui, row, flow);
                }
            });
    });
}

/// The finish line: a mark, the counts, and what the transfer took.
fn summary(ui: &mut egui::Ui, data: &TransferData, flow: egui::Color32) {
    let done = data
        .rows
        .iter()
        .filter(|r| r.state == RowState::Done)
        .count();
    let all = done == data.rows.len();
    let (icon, color) = if all {
        (Icon::Check, flow)
    } else if done == 0 {
        (Icon::Cross, theme::danger())
    } else {
        (Icon::Warning, theme::danger())
    };
    let mut parts = vec![
        if all {
            plural(done, "file")
        } else {
            format!("{done} of {} files", data.rows.len())
        },
        super::fmt_bytes(data.transferred),
    ];
    let secs = data.elapsed.as_secs_f64();
    if secs > 0.0 {
        parts.push(fmt_duration(data.elapsed));
        let bps = data.transferred as f64 / secs;
        if bps >= 1.0 {
            parts.push(format!("{}/s", super::fmt_bytes(bps as u64)));
        }
    }
    ui.horizontal(|ui| {
        let (rect, _) =
            ui.allocate_exact_size(egui::Vec2::splat(SUMMARY_ICON), egui::Sense::hover());
        ui.painter().circle_filled(
            rect.center(),
            SUMMARY_ICON / 2.0,
            color.linear_multiply(0.2),
        );
        icons::paint(
            ui.painter(),
            icon,
            rect.center(),
            SUMMARY_ICON * 0.55,
            color,
        );
        ui.vertical(|ui| {
            ui.label(
                egui::RichText::new(if all {
                    "Done"
                } else if done == 0 {
                    "Nothing arrived"
                } else {
                    "Some files did not make it"
                })
                .size(theme::ROW_FONT)
                .strong(),
            );
            ui.label(
                egui::RichText::new(parts.join(" · "))
                    .size(theme::DETAIL_FONT)
                    .color(theme::dim()),
            );
        });
    });
}

fn file_row(ui: &mut egui::Ui, row: &FileRow, flow: egui::Color32) {
    let (rect, _) = ui.allocate_exact_size(
        egui::vec2(ui.available_width(), FILE_ROW_H),
        egui::Sense::hover(),
    );
    let painter = ui.painter();
    let mark_center = egui::pos2(rect.left() + theme::ICON_SIZE / 2.0, rect.top() + 9.0);
    match row.state {
        RowState::Done => icons::paint(painter, Icon::Check, mark_center, theme::ICON_SIZE, flow),
        RowState::Failed => icons::paint(
            painter,
            Icon::Cross,
            mark_center,
            theme::ICON_SIZE,
            theme::danger(),
        ),
        RowState::Moving => {
            painter.circle_filled(mark_center, 3.0, flow);
        }
        RowState::Pending => {
            painter.circle_stroke(mark_center, 3.0, egui::Stroke::new(1.0, theme::dim()));
        }
    }
    let name_color = match row.state {
        RowState::Pending => theme::dim(),
        _ => theme::ink(),
    };
    painter.text(
        egui::pos2(rect.left() + MARK_GUTTER, rect.top() + 9.0),
        egui::Align2::LEFT_CENTER,
        &row.name,
        egui::FontId::proportional(theme::DETAIL_FONT),
        name_color,
    );
    painter.text(
        egui::pos2(rect.right() - theme::ROW_PAD, rect.top() + 9.0),
        egui::Align2::RIGHT_CENTER,
        super::fmt_bytes(row.size),
        egui::FontId::proportional(theme::DETAIL_FONT),
        theme::dim(),
    );
    // Only a moving file has a bar: a done one would be a full line of color
    // per row, and a pending one an empty track.
    if row.state == RowState::Moving {
        let bar = egui::Rect::from_min_size(
            egui::pos2(rect.left() + MARK_GUTTER, rect.top() + 19.0),
            egui::vec2(rect.width() - MARK_GUTTER - theme::ROW_PAD, FILE_BAR_H),
        );
        theme::progress(painter, bar, row.frac, flow);
    }
}

fn plural(n: usize, word: &str) -> String {
    if n == 1 {
        format!("1 {word}")
    } else {
        format!("{n} {word}s")
    }
}

/// A finished transfer's time: tenths under [`TENTHS_UNDER`] seconds, since a
/// LAN moves a ROM in less than one, then [`fmt_eta`]'s units.
fn fmt_duration(elapsed: Duration) -> String {
    const TENTHS_UNDER: f64 = 10.0;
    let secs = elapsed.as_secs_f64();
    if secs < TENTHS_UNDER {
        format!("{secs:.1}s")
    } else {
        fmt_eta(secs.round() as u64)
    }
}

/// Compact duration: "45s", "5m 05s", "1h 12m".
fn fmt_eta(secs: u64) -> String {
    match secs {
        s if s < 60 => format!("{s}s"),
        s if s < 3600 => format!("{}m {:02}s", s / 60, s % 60),
        s => format!("{}h {:02}m", s / 3600, (s % 3600) / 60),
    }
}

#[cfg(test)]
mod tests {
    use super::{fmt_duration, fmt_eta};
    use std::time::Duration;

    #[test]
    fn fmt_duration_keeps_tenths_for_a_quick_transfer() {
        assert_eq!(fmt_duration(Duration::from_millis(640)), "0.6s");
        assert_eq!(fmt_duration(Duration::from_millis(9_950)), "9.9s");
        assert_eq!(fmt_duration(Duration::from_secs(75)), "1m 15s");
    }

    #[test]
    fn fmt_eta_grows_units_with_the_wait() {
        assert_eq!(fmt_eta(0), "0s");
        assert_eq!(fmt_eta(45), "45s");
        assert_eq!(fmt_eta(60), "1m 00s");
        assert_eq!(fmt_eta(305), "5m 05s");
        assert_eq!(fmt_eta(3599), "59m 59s");
        assert_eq!(fmt_eta(4320), "1h 12m");
    }
}
