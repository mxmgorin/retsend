//! File browser renderer: breadcrumb header, dirs-first listing with
//! selection checkboxes, and a footer with the running selection total.

use super::theme;
use crate::overlay::browser::{BrowserMode, FileBrowser};
use egui_sdl2::egui;

pub fn render(root: &mut egui::Ui, browser: &FileBrowser, target_alias: &str) {
    let picking_dir = browser.mode == BrowserMode::PickDir;
    egui::Panel::top(super::TOP_PANEL_ID).show(root, |ui| {
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

    egui::Panel::bottom(super::BOTTOM_PANEL_ID).show(root, |ui| {
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
            // Built rather than a literal: X has nothing to do while a folder is
            // being chosen, and six hints is already tight on a 320px screen.
            let mut hints: Vec<(&str, &str)> = vec![
                ("Select", "Roots"),
                ("Start", if picking_dir { "Choose here" } else { "Send" }),
            ];
            if !picking_dir {
                hints.push(("X", "Dir"));
            }
            hints.push((
                "Y",
                if browser.target_is_pinned() {
                    "Unpin"
                } else {
                    "Pin"
                },
            ));
            hints.push(("B", "Up"));
            hints.push(("A", if picking_dir { "Open" } else { "Select/Open" }));
            super::home::hint_bar(ui, &hints);
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

                // A star for pinned rows, a slash for directories, a checkbox
                // for files — and both for a pinned file, whose selection state
                // still has to be readable. No checkboxes when only a directory
                // is being picked.
                let checked = browser.selected.contains_key(&entry.path);
                let (marker, marker_color) = match (entry.pinned, entry.is_dir) {
                    (true, true) => ("  ★", theme::ACCENT),
                    (true, false) if checked => ("★[x]", theme::ACCENT),
                    (true, false) => ("★[ ]", theme::ACCENT),
                    (false, true) => ("   /", theme::DIM),
                    (false, false) if picking_dir => ("", theme::DIM),
                    (false, false) if checked => ("[x]", theme::ACCENT),
                    (false, false) => ("[ ]", theme::DIM),
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
                // Pinned rows show where they lead: two cards can carry folders
                // with the same name.
                let trailing = if entry.pinned {
                    truncate_middle(&entry.path.display().to_string(), 40)
                } else if entry.is_dir {
                    String::new()
                } else {
                    super::fmt_bytes(entry.size)
                };
                if !trailing.is_empty() {
                    painter.text(
                        rect.right_center() - egui::vec2(padding, 0.0),
                        egui::Align2::RIGHT_CENTER,
                        trailing,
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
