//! File browser renderer: breadcrumb header, dirs-first listing with
//! selection checkboxes, and a footer with the running selection total.

use super::{theme, truncate_middle};
use crate::overlay::browser::{BrowserMode, DirPurpose, FileBrowser};
use egui_sdl2::egui;

/// Listing row height — denser than [`theme::ROW_HEIGHT`] so a folder shows
/// more files per screen.
const ENTRY_HEIGHT: f32 = 30.0;

/// `deadline_secs` is set only while the browser is picking a destination for
/// a parked incoming request: the modal (and its countdown bar) is hidden
/// behind us, so the seconds left have to show up here.
pub fn render(
    root: &mut egui::Ui,
    browser: &FileBrowser,
    target_alias: &str,
    deadline_secs: Option<u32>,
) {
    let picking_dir = browser.mode == BrowserMode::PickDir;
    let for_incoming = picking_dir && browser.dir_purpose == DirPurpose::Incoming;
    egui::Panel::top(super::TOP_PANEL_ID).show(root, |ui| {
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            let title = match (picking_dir, for_incoming) {
                (_, true) => format!("Save files from {target_alias} here"),
                (true, false) => "Choose the save folder".to_string(),
                (false, _) => format!("Send to {target_alias}"),
            };
            ui.label(egui::RichText::new(title).size(theme::ROW_FONT).strong());
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if let Some(secs) = deadline_secs {
                    ui.label(
                        egui::RichText::new(format!("{secs}s"))
                            .size(theme::DETAIL_FONT)
                            .color(theme::ACCENT),
                    );
                }
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
            let start_hint = match (picking_dir, for_incoming) {
                (_, true) => "Save here",
                (true, false) => "Choose here",
                (false, _) => "Send",
            };
            let mut hints: Vec<(&str, &str)> = vec![("Select", "Roots"), ("Start", start_hint)];
            if !picking_dir {
                hints.push(("X", "All"));
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
        // Virtualized: only the visible slice is laid out and painted — ROM
        // folders can hold thousands of entries.
        let spacing = ui.spacing().item_spacing.y;
        let step = ENTRY_HEIGHT + spacing;
        let total = browser.entries.len();
        egui::ScrollArea::vertical().show_viewport(ui, |ui, viewport| {
            ui.set_height(step * total as f32 - spacing);
            ui.set_width(ui.available_width());
            let origin = ui.min_rect().left_top();
            let width = ui.available_width();
            let row_rect = |i: usize| {
                egui::Rect::from_min_size(
                    origin + egui::vec2(0.0, i as f32 * step),
                    egui::vec2(width, ENTRY_HEIGHT),
                )
            };
            // Keeps the cursor visible even when it jumped past the slice.
            if browser.cursor < total {
                ui.scroll_to_rect(row_rect(browser.cursor), None);
            }
            let first = (viewport.min.y / step).max(0.0) as usize;
            let last = ((viewport.max.y / step).ceil() as usize + 1).min(total);
            for i in first..last {
                let entry = &browser.entries[i];
                let rect = row_rect(i);
                if browser.cursor == i {
                    ui.painter().rect(
                        rect,
                        4.0,
                        theme::ACCENT.linear_multiply(0.30),
                        egui::Stroke::new(1.0, theme::ACCENT),
                        egui::StrokeKind::Inside,
                    );
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
