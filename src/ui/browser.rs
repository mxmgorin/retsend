//! File browser renderer: breadcrumb header, dirs-first listing with
//! checkboxes and folder/file marks, and a footer with the running selection
//! total. Colored by what the pick is for: coral when choosing files to send,
//! teal when choosing where received files land.

use super::icons::{self, Icon};
use super::{theme, truncate_middle};
use crate::app::AppCommand;
use crate::overlay::browser::{BrowserMode, DirPurpose, FileBrowser};
use egui_sdl2::egui;

/// Listing row height — denser than [`theme::ROW_HEIGHT`] so a folder shows
/// more files per screen.
const ENTRY_HEIGHT: f32 = 30.0;

/// The pinned band: an accent wash faint enough to stay under the cursor's own
/// fill, the rule closing it, and the padding it grows by past its rows.
const BAND_TINT: f32 = 0.10;
const RULE_TINT: f32 = 0.55;
const BAND_INSET: f32 = 2.0;

/// Width of a mark column (checkbox, folder/file icon) in a row.
const MARK_COL: f32 = 22.0;
/// Side of a row's icons.
const MARK_SIZE: f32 = theme::ICON_SIZE;

/// `deadline_secs` is set only while the browser is picking a destination for
/// a parked incoming request: the modal (and its countdown bar) is hidden
/// behind us, so the seconds left have to show up here.
pub fn render(
    root: &mut egui::Ui,
    browser: &FileBrowser,
    target_alias: &str,
    deadline_secs: Option<u32>,
    taps: &mut Vec<AppCommand>,
) {
    let picking_dir = browser.mode == BrowserMode::PickDir;
    let for_incoming = picking_dir && browser.dir_purpose == DirPurpose::Incoming;
    // Files picked here leave the device; a folder picked here takes files in.
    let flow = theme::flow_color(!picking_dir);
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
                            .color(super::prompt::countdown_color(secs)),
                    );
                }
                ui.label(
                    egui::RichText::new(truncate_middle(&browser.cwd.display().to_string(), 48))
                        .size(theme::DETAIL_FONT)
                        .color(theme::dim()),
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
                        .color(flow),
                );
                ui.label(egui::RichText::new("·").color(theme::dim()));
            }
            // Built rather than a literal: X has nothing to do while a folder is
            // being chosen, and six hints is already tight on a 320px screen.
            let start_hint = match (picking_dir, for_incoming) {
                (_, true) => "Save here",
                (true, false) => "Choose here",
                (false, _) => "Send",
            };
            let mut hints: Vec<super::home::Hint> = vec![
                ("Select", "Roots", Some(AppCommand::ReAnnounce)),
                ("Start", start_hint, Some(AppCommand::Start)),
            ];
            if !picking_dir {
                let what = if browser.cursor_is_dir() {
                    "Folder"
                } else {
                    "All"
                };
                hints.push(("X", what, Some(AppCommand::Alt)));
            }
            hints.push((
                "Y",
                if browser.target_is_pinned() {
                    "Unpin"
                } else {
                    "Pin"
                },
                Some(AppCommand::TogglePin),
            ));
            hints.push(("B", "Up", Some(AppCommand::Back)));
            hints.push(("A", if picking_dir { "Open" } else { "Select/Open" }, None));
            super::home::hint_bar(ui, &hints, taps);
        });
        ui.add_space(4.0);
    });

    egui::CentralPanel::default().show(root, |ui| {
        if browser.entries.is_empty() {
            ui.centered_and_justified(|ui| {
                ui.label(
                    egui::RichText::new("Empty folder")
                        .size(theme::ROW_FONT)
                        .color(theme::dim()),
                );
            });
            return;
        }
        // Virtualized: only the visible slice is laid out and painted — ROM
        // folders can hold thousands of entries.
        let spacing = ui.spacing().item_spacing.y;
        let step = ENTRY_HEIGHT + spacing;
        let total = browser.entries.len();
        egui::ScrollArea::vertical()
            .id_salt("browser")
            .show_viewport(ui, |ui, viewport| {
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
                // Keyed by folder too: a new listing starts at row 0 as well.
                if browser.cursor < total {
                    super::scroll_to_cursor(
                        ui,
                        ("browser", &browser.cwd, browser.cursor),
                        row_rect(browser.cursor),
                    );
                }
                // Pins belong to other folders: a band and a closing rule keep them
                // from reading as rows of the folder being looked at.
                let pins = browser.pinned_rows();
                if pins > 0 {
                    let band = row_rect(0).union(row_rect(pins - 1));
                    ui.painter().rect_filled(
                        band.expand2(egui::vec2(0.0, BAND_INSET)),
                        4.0,
                        flow.linear_multiply(BAND_TINT),
                    );
                    if pins < total {
                        ui.painter().hline(
                            band.x_range(),
                            band.max.y + spacing * 0.5,
                            egui::Stroke::new(1.0, theme::dim().linear_multiply(RULE_TINT)),
                        );
                    }
                }
                let first = (viewport.min.y / step).max(0.0) as usize;
                let last = ((viewport.max.y / step).ceil() as usize + 1).min(total);
                // Hit-tested, not sensed: virtualized rows are painted from rects,
                // not allocated.
                let tap = super::home::tap_pos(ui);
                for i in first..last {
                    let entry = &browser.entries[i];
                    let rect = row_rect(i);
                    if tap.is_some_and(|pos| rect.contains(pos)) {
                        taps.push(AppCommand::PickRow(i));
                        taps.push(AppCommand::Confirm);
                    }
                    if browser.cursor == i {
                        theme::highlight(ui.painter(), rect, flow);
                    }
                    let painter = ui.painter();

                    // A checkbox for whatever can be picked (none when only a directory
                    // is), then a star, folder or file mark.
                    let picked = browser.selected.get(&entry.path);
                    // Checked, but dimmed: a picked folder put it there.
                    let covered = picked.is_none() && browser.covered_by(&entry.path).is_some();
                    let checked = picked.is_some() || covered;
                    let mut x = rect.left() + theme::ROW_PAD + MARK_SIZE / 2.0;
                    if !picking_dir {
                        let (icon, color) = match (checked, covered) {
                            (true, true) => (Icon::CheckboxOn, theme::dim()),
                            (true, false) => (Icon::CheckboxOn, flow),
                            (false, _) => (Icon::CheckboxOff, theme::dim()),
                        };
                        icons::paint(
                            painter,
                            icon,
                            egui::pos2(x, rect.center().y),
                            MARK_SIZE,
                            color,
                        );
                        x += MARK_COL;
                    }
                    let (icon, color) = match (entry.pinned, entry.is_dir) {
                        (true, _) => (Icon::Star, flow),
                        (false, true) => (Icon::Folder, theme::dim()),
                        (false, false) => (Icon::File, theme::dim()),
                    };
                    icons::paint(
                        painter,
                        icon,
                        egui::pos2(x, rect.center().y),
                        MARK_SIZE,
                        color,
                    );
                    x += MARK_COL;
                    painter.text(
                        egui::pos2(x - MARK_SIZE / 2.0, rect.center().y),
                        egui::Align2::LEFT_CENTER,
                        &entry.name,
                        egui::FontId::proportional(theme::ROW_FONT),
                        theme::ink(),
                    );
                    // A pinned row shows where it leads, two cards can carry a folder of
                    // one name; a picked folder shows what it weighs.
                    let trailing = if entry.pinned {
                        truncate_middle(&entry.path.display().to_string(), 40)
                    } else if entry.is_dir {
                        picked.map_or_else(String::new, |p| super::fmt_bytes(p.bytes))
                    } else {
                        super::fmt_bytes(entry.size)
                    };
                    if !trailing.is_empty() {
                        painter.text(
                            rect.right_center() - egui::vec2(theme::ROW_PAD, 0.0),
                            egui::Align2::RIGHT_CENTER,
                            trailing,
                            egui::FontId::proportional(theme::DETAIL_FONT),
                            theme::dim(),
                        );
                    }
                }
            });
    });
}
