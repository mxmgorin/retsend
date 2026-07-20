//! OSK renderer: the buffer with a caret on top, the key grid below,
//! anchored to the bottom of the screen.

use super::theme;
use crate::overlay::osk::{Key, Osk};
use egui_sdl2::egui;

pub fn render(ctx: &egui::Context, osk: &Osk) {
    let screen = ctx.content_rect();
    egui::Area::new(egui::Id::new("osk"))
        .anchor(egui::Align2::CENTER_BOTTOM, egui::vec2(0.0, -8.0))
        .show(ctx, |ui| {
            egui::Frame::new()
                .fill(theme::PANEL_FILL)
                .stroke(egui::Stroke::new(1.0, theme::ACCENT))
                .corner_radius(8.0)
                .inner_margin(egui::Margin::same(10))
                .show(ui, |ui| {
                    ui.set_width((screen.width() * 0.9).min(520.0));

                    // Buffer line with a caret.
                    ui.label(
                        egui::RichText::new(format!("{}▏", osk.buffer))
                            .size(theme::ROW_FONT)
                            .strong(),
                    );
                    ui.add_space(6.0);

                    let key_h = 30.0;
                    let gap = 4.0;
                    for (row_index, row) in osk.rows().iter().enumerate() {
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = gap;
                            let width = ui.available_width();
                            let key_w = (width - gap * (row.len() as f32 - 1.0)) / row.len() as f32;
                            for (col_index, key) in row.iter().enumerate() {
                                let (rect, _) = ui.allocate_exact_size(
                                    egui::vec2(key_w, key_h),
                                    egui::Sense::hover(),
                                );
                                let selected = osk.row == row_index && osk.col == col_index;
                                let (fill, stroke) = if selected {
                                    (
                                        theme::ACCENT.linear_multiply(0.30),
                                        egui::Stroke::new(1.0, theme::ACCENT),
                                    )
                                } else {
                                    (
                                        egui::Color32::from_gray(0x24),
                                        egui::Stroke::new(1.0, egui::Color32::from_gray(0x38)),
                                    )
                                };
                                ui.painter().rect(
                                    rect,
                                    4.0,
                                    fill,
                                    stroke,
                                    egui::StrokeKind::Inside,
                                );
                                let label = match key {
                                    Key::Char(c) => c.to_string(),
                                    Key::Space => "space".to_string(),
                                    Key::Backspace => "del".to_string(),
                                    Key::Layer => osk.layer_name().to_string(),
                                    Key::Ok => "OK".to_string(),
                                };
                                let color = match key {
                                    Key::Ok => theme::ACCENT,
                                    Key::Char(_) => ui.visuals().text_color(),
                                    _ => theme::DIM,
                                };
                                ui.painter().text(
                                    rect.center(),
                                    egui::Align2::CENTER_CENTER,
                                    label,
                                    egui::FontId::proportional(theme::DETAIL_FONT + 1.0),
                                    color,
                                );
                            }
                        });
                    }

                    ui.add_space(6.0);
                    super::home::hint_bar(
                        ui,
                        &[
                            ("A", "Type"),
                            ("B", "Erase"),
                            ("Start", "OK"),
                            ("Select", "Layer"),
                        ],
                    );
                });
        });
}
