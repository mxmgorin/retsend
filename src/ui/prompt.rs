//! The incoming-request modal: who is sending what, where it will land, and
//! a countdown to the automatic decline. Rendered over whatever screen is
//! underneath whenever a prepare-upload is parked.

use super::icons::{self, Icon};
use super::theme;
use crate::app::AppCommand;
use crate::net::server::DECISION_TIMEOUT;
use egui_sdl2::egui;

/// Files shown by name before collapsing into "…and N more".
pub(crate) const SHOWN_FILES: usize = 8;
/// Destination folders shown before collapsing into "…and N more folders".
/// Save routes can spread one request over as many folders as it has
/// extensions, and the modal still has to fit on a 480 px screen.
pub(crate) const SHOWN_DESTS: usize = 4;
/// With this many seconds left, the countdown turns red.
pub(crate) const URGENT_SECS: u32 = 10;
/// Height of the countdown bar.
const COUNTDOWN_H: f32 = 4.0;

pub struct PromptData {
    pub sender: String,
    /// What the sender announced itself as.
    pub icon: Icon,
    /// (name, size) — first [`SHOWN_FILES`] of the request, name-sorted.
    pub files: Vec<(String, u64)>,
    pub hidden: usize,
    pub count: usize,
    pub total_bytes: u64,
    /// Distinct destination folders — first [`SHOWN_DESTS`] of them.
    pub dests: Vec<String>,
    pub hidden_dests: usize,
    /// 1.0 → fresh, 0.0 → deadline; drives the countdown bar.
    pub remaining: f32,
}

impl PromptData {
    /// Whole seconds left before the automatic decline.
    pub fn seconds_left(&self) -> u32 {
        (self.remaining * DECISION_TIMEOUT.as_secs_f32())
            .max(0.0)
            .ceil() as u32
    }
}

/// The countdown's color: the accent until the last seconds, then red.
pub(crate) fn countdown_color(seconds_left: u32) -> egui::Color32 {
    if seconds_left <= URGENT_SECS {
        theme::DANGER
    } else {
        theme::ACCENT
    }
}

pub fn render(ctx: &egui::Context, data: &PromptData, taps: &mut Vec<AppCommand>) {
    // Dim the screen underneath so the modal reads as blocking.
    let screen = ctx.content_rect();
    egui::Area::new(egui::Id::new("prompt_backdrop"))
        .fixed_pos(screen.min)
        .show(ctx, |ui| {
            ui.painter().rect_filled(screen, 0.0, theme::SCRIM);
        });

    let countdown = countdown_color(data.seconds_left());
    egui::Area::new(egui::Id::new("prompt"))
        .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
        .show(ctx, |ui| {
            theme::card_frame(theme::ACCENT).show(ui, |ui| {
                ui.set_width((screen.width() * 0.75).min(460.0));

                ui.horizontal(|ui| {
                    let (rect, _) = ui.allocate_exact_size(
                        egui::Vec2::splat(theme::ICON_SIZE + 6.0),
                        egui::Sense::hover(),
                    );
                    icons::paint(
                        ui.painter(),
                        data.icon,
                        rect.center(),
                        theme::ICON_SIZE + 4.0,
                        theme::ACCENT,
                    );
                    ui.label(
                        egui::RichText::new(format!(
                            "{} wants to send {} ({})",
                            data.sender,
                            plural(data.count, "file"),
                            super::fmt_bytes(data.total_bytes),
                        ))
                        .size(theme::ROW_FONT)
                        .strong(),
                    );
                });
                ui.add_space(8.0);

                for (name, size) in &data.files {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new(name).size(theme::DETAIL_FONT));
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(
                                egui::RichText::new(super::fmt_bytes(*size))
                                    .size(theme::DETAIL_FONT)
                                    .color(theme::DIM),
                            );
                        });
                    });
                }
                if data.hidden > 0 {
                    ui.label(
                        egui::RichText::new(format!("…and {} more", data.hidden))
                            .size(theme::DETAIL_FONT)
                            .color(theme::DIM),
                    );
                }

                ui.add_space(8.0);
                // One line per folder the save routes picked, truncated: a wrapped
                // path pushes the arrow onto a line of its own.
                for dest in &data.dests {
                    ui.label(
                        egui::RichText::new(format!(
                            "→ {}",
                            super::truncate_middle(dest, super::PATH_CHARS)
                        ))
                        .size(theme::DETAIL_FONT)
                        .color(theme::DIM),
                    );
                }
                if data.hidden_dests > 0 {
                    ui.label(
                        egui::RichText::new(format!(
                            "…and {}",
                            plural(data.hidden_dests, "more folder")
                        ))
                        .size(theme::DETAIL_FONT)
                        .color(theme::DIM),
                    );
                }
                ui.add_space(8.0);

                // Countdown to auto-decline, with the seconds beside it once
                // they are few.
                ui.horizontal(|ui| {
                    let secs = data.seconds_left();
                    let label_w = if secs <= URGENT_SECS { 28.0 } else { 0.0 };
                    let (rect, _) = ui.allocate_exact_size(
                        egui::vec2(ui.available_width() - label_w, COUNTDOWN_H),
                        egui::Sense::hover(),
                    );
                    theme::progress(ui.painter(), rect, data.remaining, countdown);
                    if secs <= URGENT_SECS {
                        ui.label(
                            egui::RichText::new(format!("{secs}s"))
                                .size(theme::DETAIL_FONT)
                                .color(countdown),
                        );
                    }
                });

                ui.add_space(10.0);
                super::home::hint_bar(
                    ui,
                    &[
                        ("B", "Decline", Some(AppCommand::Back)),
                        ("X", "Save to…", Some(AppCommand::Alt)),
                        ("A", "Accept", Some(AppCommand::Confirm)),
                    ],
                    taps,
                );
            });
        });
}

fn plural(n: usize, word: &str) -> String {
    if n == 1 {
        format!("1 {word}")
    } else {
        format!("{n} {word}s")
    }
}
