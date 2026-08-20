//! The transfer screen: overall progress with speed/ETA, per-file rows, the
//! finish summary, and the two-step cancel confirmation.

use super::theme;
use crate::app::AppCommand;
use egui_sdl2::egui;

pub struct TransferData {
    /// Built by `ui::mod` from the session's direction and phase.
    pub title: String,
    pub finished: bool,
    pub transferred: u64,
    pub total: u64,
    pub speed_bps: Option<f64>,
    pub rows: Vec<FileRow>,
    pub confirm_cancel: bool,
}

pub struct FileRow {
    pub name: String,
    pub size: u64,
    /// "√" done, "×" failed, "" pending/receiving.
    pub glyph: &'static str,
    /// 0.0..=1.0 of this file.
    pub frac: f32,
}

pub fn render(root: &mut egui::Ui, data: &TransferData, taps: &mut Vec<AppCommand>) {
    egui::Panel::top(super::TOP_PANEL_ID).show(root, |ui| {
        ui.add_space(6.0);
        ui.label(
            egui::RichText::new(&data.title)
                .size(theme::ROW_FONT + 2.0)
                .strong(),
        );
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
        // Overall bar + the numbers line.
        let frac = if data.total > 0 {
            data.transferred as f32 / data.total as f32
        } else {
            1.0
        };
        ui.add(
            egui::ProgressBar::new(frac)
                .desired_height(10.0)
                .fill(theme::ACCENT),
        );
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
        ui.label(
            egui::RichText::new(line)
                .size(theme::DETAIL_FONT)
                .color(theme::DIM),
        );
        ui.add_space(8.0);

        egui::ScrollArea::vertical().show(ui, |ui| {
            for row in &data.rows {
                ui.horizontal(|ui| {
                    let glyph_color = match row.glyph {
                        "√" => theme::ACCENT,
                        "×" => egui::Color32::from_rgb(0xd0, 0x60, 0x60),
                        _ => theme::DIM,
                    };
                    let glyph = if row.glyph.is_empty() {
                        format!("{:>3.0}%", row.frac * 100.0)
                    } else {
                        format!("  {} ", row.glyph)
                    };
                    ui.label(
                        egui::RichText::new(glyph)
                            .size(theme::DETAIL_FONT)
                            .monospace()
                            .color(glyph_color),
                    );
                    ui.label(egui::RichText::new(&row.name).size(theme::DETAIL_FONT));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(
                            egui::RichText::new(super::fmt_bytes(row.size))
                                .size(theme::DETAIL_FONT)
                                .color(theme::DIM),
                        );
                    });
                });
            }
        });
    });
}

/// Compact ETA: "45s", "5m 05s", "1h 12m".
fn fmt_eta(secs: u64) -> String {
    match secs {
        s if s < 60 => format!("{s}s"),
        s if s < 3600 => format!("{}m {:02}s", s / 60, s % 60),
        s => format!("{}h {:02}m", s / 3600, (s % 3600) / 60),
    }
}

#[cfg(test)]
mod tests {
    use super::fmt_eta;

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
