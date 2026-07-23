//! File browser renderer: breadcrumb header, dirs-first listing with
//! selection checkboxes, and a footer with the running selection total.

use super::theme;
use crate::overlay::browser::{BrowserMode, FileBrowser};
use egui_sdl2::egui;

pub fn render(root: &mut egui::Ui, browser: &FileBrowser, target_alias: &str) {
    let picking_dir = browser.mode == BrowserMode::PickDir;
    egui::Panel::top("browser_header").show(root, |ui| {
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            let title = if picking_dir {
                "Choose the save folder".to_string()
            } else {
                format!("Send to {target_alias}")
            };
            ui.label(egui::RichText::new(title).size(theme::ROW_FONT).strong());
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(
                    egui::RichText::new(truncate_middle(&browser.cwd.display().to_string(), 48))
                        .size(theme::DETAIL_FONT)
                        .color(theme::DIM),
                );
            });
        });
        ui.add_space(6.0);
    });

    egui::Panel::bottom("browser_footer").show(root, |ui| {
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            let (count, bytes) = browser.selection_totals();
            if count > 0 {
                ui.label(
                    egui::RichText::new(format!("{count} selected · {}", super::fmt_bytes(bytes)))
                        .size(theme::DETAIL_FONT)
                        .color(theme::ACCENT),
                );
                ui.label(egui::RichText::new("·").color(theme::DIM));
            }
            super::home::hint_bar(
                ui,
                &[
                    ("A", if picking_dir { "Open" } else { "Select/Open" }),
                    ("B", "Up"),
                    ("Start", if picking_dir { "Choose here" } else { "Send" }),
                    ("Select", "Roots"),
                ],
            );
        });
        ui.add_space(4.0);
    });

    egui::CentralPanel::default().show(root, |ui| {
        if browser.entries.is_empty() {
            ui.centered_and_justified(|ui| {
                ui.label(
                    egui::RichText::new("Empty folder")
                        .size(theme::ROW_FONT)
                        .color(theme::DIM),
                );
            });
            return;
        }
        egui::ScrollArea::vertical().show(ui, |ui| {
            for (i, entry) in browser.entries.iter().enumerate() {
                let selected = browser.cursor == i;
                let desired = egui::vec2(ui.available_width(), 30.0);
                let (rect, response) = ui.allocate_exact_size(desired, egui::Sense::hover());
                if selected {
                    ui.painter().rect(
                        rect,
                        4.0,
                        theme::ACCENT.linear_multiply(0.30),
                        egui::Stroke::new(1.0, theme::ACCENT),
                        egui::StrokeKind::Inside,
                    );
                    response.scroll_to_me(None);
                }
                let painter = ui.painter();
                let padding = 10.0;

                // Checkbox for files, a slash marker for directories; no
                // checkboxes when only a directory is being picked.
                let (marker, marker_color) = if entry.is_dir {
                    ("   /", theme::DIM)
                } else if picking_dir {
                    ("", theme::DIM)
                } else if browser.selected.contains_key(&entry.path) {
                    ("[x]", theme::ACCENT)
                } else {
                    ("[ ]", theme::DIM)
                };
                painter.text(
                    rect.left_center() + egui::vec2(padding, 0.0),
                    egui::Align2::LEFT_CENTER,
                    marker,
                    egui::FontId::monospace(theme::DETAIL_FONT),
                    marker_color,
                );
                painter.text(
                    rect.left_center() + egui::vec2(padding + 36.0, 0.0),
                    egui::Align2::LEFT_CENTER,
                    &entry.name,
                    egui::FontId::proportional(theme::ROW_FONT),
                    ui.visuals().text_color(),
                );
                if !entry.is_dir {
                    painter.text(
                        rect.right_center() - egui::vec2(padding, 0.0),
                        egui::Align2::RIGHT_CENTER,
                        super::fmt_bytes(entry.size),
                        egui::FontId::proportional(theme::DETAIL_FONT),
                        theme::DIM,
                    );
                }
            }
        });
    });
}

/// Keep the tail of a long path visible: `/very/…/deep/folder`.
fn truncate_middle(s: &str, max_chars: usize) -> String {
    let count = s.chars().count();
    if count <= max_chars {
        return s.to_string();
    }
    let keep = max_chars.saturating_sub(1);
    let head = keep / 3;
    let tail = keep - head;
    let head_str: String = s.chars().take(head).collect();
    let tail_str: String = s.chars().skip(count - tail).collect();
    format!("{head_str}…{tail_str}")
}
