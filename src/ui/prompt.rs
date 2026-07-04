//! The incoming-request modal: who is sending what, where it will land, and
//! a countdown to the automatic decline. Rendered over whatever screen is
//! underneath whenever a prepare-upload is parked.

use super::theme;
use egui_sdl2::egui;

/// Files shown by name before collapsing into "…and N more".
pub(crate) const SHOWN_FILES: usize = 8;

pub struct PromptData {
    pub sender: String,
    /// (name, size) — first [`SHOWN_FILES`] of the request, name-sorted.
    pub files: Vec<(String, u64)>,
    pub hidden: usize,
    pub count: usize,
    pub total_bytes: u64,
    pub dest: String,
    /// 1.0 → fresh, 0.0 → deadline; drives the countdown bar.
    pub remaining: f32,
}

pub fn render(ctx: &egui::Context, data: &PromptData) {
    // Dim the screen underneath so the modal reads as blocking.
    let screen = ctx.content_rect();
    egui::Area::new(egui::Id::new("prompt_backdrop"))
        .fixed_pos(screen.min)
        .show(ctx, |ui| {
            ui.painter()
                .rect_filled(screen, 0.0, egui::Color32::from_black_alpha(160));
        });

    egui::Area::new(egui::Id::new("prompt"))
        .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
        .show(ctx, |ui| {
            egui::Frame::new()
                .fill(theme::PANEL_FILL)
                .stroke(egui::Stroke::new(1.0, theme::ACCENT))
                .corner_radius(8.0)
                .inner_margin(egui::Margin::same(14))
                .show(ui, |ui| {
                    ui.set_width((screen.width() * 0.75).min(460.0));

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
                    ui.add_space(8.0);

                    for (name, size) in &data.files {
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new(name).size(theme::DETAIL_FONT));
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    ui.label(
                                        egui::RichText::new(super::fmt_bytes(*size))
                                            .size(theme::DETAIL_FONT)
                                            .color(theme::DIM),
                                    );
                                },
                            );
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
                    ui.label(
                        egui::RichText::new(format!("→ {}", data.dest))
                            .size(theme::DETAIL_FONT)
                            .color(theme::DIM),
                    );
                    ui.add_space(8.0);

                    // Countdown to auto-decline.
                    let (rect, _) = ui.allocate_exact_size(
                        egui::vec2(ui.available_width(), 4.0),
                        egui::Sense::hover(),
                    );
                    ui.painter()
                        .rect_filled(rect, 2.0, egui::Color32::from_gray(0x33));
                    let mut lit = rect;
                    lit.set_width(rect.width() * data.remaining.clamp(0.0, 1.0));
                    ui.painter().rect_filled(lit, 2.0, theme::ACCENT);

                    ui.add_space(10.0);
                    super::home::hint_bar(ui, &[("A", "Accept"), ("B", "Decline")]);
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
