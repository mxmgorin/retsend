//! OSK renderer: the buffer with a caret on top, the key grid below,
//! anchored to the bottom of the screen.

use super::theme;
use crate::app::AppCommand;
use crate::overlay::osk::{Key, Osk};
use egui_sdl2::egui;

const KEY_H: f32 = 30.0;
const KEY_GAP: f32 = 4.0;
const KEY_RADIUS: f32 = 5.0;

pub fn render(ctx: &egui::Context, osk: &Osk, taps: &mut Vec<AppCommand>) {
    let screen = ctx.content_rect();
    egui::Area::new(egui::Id::new("osk"))
        .anchor(egui::Align2::CENTER_BOTTOM, egui::vec2(0.0, -8.0))
        .show(ctx, |ui| {
            theme::card_frame(theme::accent()).show(ui, |ui| {
                ui.set_width((screen.width() * 0.9).min(520.0));

                ui.label(
                    egui::RichText::new(osk.target.prompt())
                        .size(theme::DETAIL_FONT)
                        .color(theme::dim()),
                );
                ui.label(
                    egui::RichText::new(format!("{}▏", osk.buffer))
                        .size(theme::ROW_FONT)
                        .strong(),
                );
                ui.add_space(6.0);

                for (row_index, row) in osk.rows().iter().enumerate() {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = KEY_GAP;
                        let width = ui.available_width();
                        let key_w = (width - KEY_GAP * (row.len() as f32 - 1.0)) / row.len() as f32;
                        for (col_index, key) in row.iter().enumerate() {
                            let (rect, response) = ui.allocate_exact_size(
                                egui::vec2(key_w, KEY_H),
                                egui::Sense::click(),
                            );
                            if response.clicked() {
                                taps.push(AppCommand::PickKey {
                                    row: row_index,
                                    col: col_index,
                                });
                                taps.push(AppCommand::Confirm);
                            }
                            let selected = osk.row == row_index && osk.col == col_index;
                            if selected {
                                theme::highlight(ui.painter(), rect, theme::accent());
                            } else {
                                ui.painter().rect(
                                    rect,
                                    KEY_RADIUS,
                                    theme::surface(),
                                    egui::Stroke::new(1.0, theme::border()),
                                    egui::StrokeKind::Inside,
                                );
                            }
                            let label = match key {
                                Key::Char(c) => c.to_string(),
                                Key::Space => "space".to_string(),
                                Key::Backspace => "del".to_string(),
                                Key::Layer => osk.layer_name().to_string(),
                                Key::Ok => "OK".to_string(),
                            };
                            let color = match key {
                                Key::Ok => theme::accent(),
                                Key::Char(_) => theme::ink(),
                                _ => theme::dim(),
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
                        ("Select", "Layer", Some(AppCommand::ReAnnounce)),
                        ("Start", "OK", Some(AppCommand::Start)),
                        ("X", "Erase", Some(AppCommand::Alt)),
                        ("B", "Back", Some(AppCommand::Back)),
                        ("A", "Type", None),
                    ],
                    taps,
                );
            });
        });
}
