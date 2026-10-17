//! Settings screen renderer: a scrolling list of name/value rows under
//! section headers, toggles drawn as switches, the current row's action in the
//! footer.

use super::icons::{self, Icon};
use super::theme;
use crate::app::{AppCommand, Direction};
use crate::config::AppConfig;
use crate::overlay::settings::Settings;
use egui_sdl2::egui;

/// What A does on a row, and how its value is drawn.
enum Kind {
    /// A opens an editor; the value is text.
    Edit(&'static str),
    /// A flips it; the value is a switch with a word beside it.
    Toggle(bool),
    /// ◂ ▸ walk the value; the words they are hinted with.
    Stepper(&'static str, &'static str),
}

/// Headers, by the index of the row they sit above. Order matches
/// `crate::overlay::settings::ROWS`.
const SECTIONS: [(usize, &str); 6] = [
    (0, "Device"),
    (1, "Receiving"),
    (5, "Save routes"),
    (7, "Network"),
    (8, "Display"),
    (11, "Controls"),
];

/// The switch: its box, the knob inside, and the gap to the word beside it.
const SWITCH_SIZE: egui::Vec2 = egui::vec2(30.0, 16.0);
const KNOB_INSET: f32 = 2.5;
const SWITCH_GAP: f32 = 10.0;
/// A stepper's chevrons, and the gap between each and the value.
const CHEVRON_SIZE: f32 = theme::ICON_SIZE;
const CHEVRON_GAP: f32 = 4.0;
/// Height of a section header row.
const HEADER_H: f32 = 24.0;

pub fn render(
    root: &mut egui::Ui,
    state: &Settings,
    config: &AppConfig,
    actual_port: u16,
    taps: &mut Vec<AppCommand>,
) {
    // Order matches `crate::overlay::settings::ROWS`.
    let rows: [(&str, String, Kind); crate::overlay::settings::ROW_COUNT] = [
        (
            "Device name",
            config.device.alias.clone(),
            Kind::Edit("Edit"),
        ),
        (
            "Save folder",
            config.transfer.save_dir.clone(),
            Kind::Edit("Choose"),
        ),
        (
            "Quick save",
            if config.transfer.auto_accept {
                "accept without asking".into()
            } else {
                "ask first".into()
            },
            Kind::Toggle(config.transfer.auto_accept),
        ),
        (
            "If a file exists",
            if config.transfer.overwrite {
                "replace".into()
            } else {
                "keep both — save as name (1)".into()
            },
            Kind::Toggle(config.transfer.overwrite),
        ),
        (
            "Received folders",
            if config.transfer.keep_folders {
                "keep the sender's structure".into()
            } else {
                "flatten into the save routes".into()
            },
            Kind::Toggle(config.transfer.keep_folders),
        ),
        (
            "Auto save routes",
            auto_routes_value(config.transfer.auto_routes, state.auto_route_count),
            Kind::Toggle(config.transfer.auto_routes),
        ),
        (
            "Save routes",
            match config.transfer.routes.len() {
                0 => "none".into(),
                1 => "1 extension".into(),
                n => format!("{n} extensions"),
            },
            Kind::Edit("Edit"),
        ),
        (
            "Port",
            port_label(config.network.port, actual_port, state.port_dirty),
            Kind::Edit("Edit"),
        ),
        (
            "Theme",
            config.display.theme.name().into(),
            Kind::Stepper("Previous", "Next"),
        ),
        (
            "UI scale",
            scale_label(config.display.scale),
            Kind::Stepper("Smaller", "Bigger"),
        ),
        (
            "Idle animation",
            if config.display.idle_animation {
                "the orbit turns while waiting".into()
            } else {
                "still".into()
            },
            Kind::Toggle(config.display.idle_animation),
        ),
        (
            "Swap A/B and X/Y",
            if config.input.swap_abxy {
                "A is the east button, X the north".into()
            } else {
                "A is the south button, X the west".into()
            },
            Kind::Toggle(config.input.swap_abxy),
        ),
        (
            "Keys",
            match config.input.keys.changed() {
                0 => "defaults".into(),
                1 => "1 button changed".into(),
                n => format!("{n} buttons changed"),
            },
            Kind::Edit("Edit"),
        ),
        (
            "About",
            format!("retsend {}", env!("CARGO_PKG_VERSION")),
            Kind::Edit("Open"),
        ),
    ];

    // No title panel: the tab bar already names the tab, and a top panel
    // absent on the other tabs would shift egui's panel-id sequence.
    egui::Panel::bottom(super::BOTTOM_PANEL_ID).show(root, |ui| {
        ui.add_space(4.0);
        // The row's action, where every other screen puts its buttons.
        let shown = state.rows();
        let at = shown[state.cursor.min(shown.len() - 1)].0;
        let mut hints: Vec<super::home::Hint> = match rows[at].2 {
            Kind::Edit(action) => vec![
                ("← →", "Tabs", None),
                ("A", action, Some(AppCommand::Confirm)),
            ],
            Kind::Toggle(_) => vec![
                ("← →", "Tabs", None),
                ("A", "Toggle", Some(AppCommand::Confirm)),
            ],
            // A stepper row spends ◂ ▸ on its own value, so it names them —
            // and naming them is what lets a screen with no pad reach them.
            Kind::Stepper(back, on) => vec![
                ("◂", back, Some(AppCommand::Nav(Direction::Left))),
                ("▸", on, Some(AppCommand::Nav(Direction::Right))),
                ("L1 R1", "Tabs", None),
            ],
        };
        if crate::app::back_quits() {
            hints.push(("B", "Quit", Some(AppCommand::Back)));
        }
        super::home::hint_bar(ui, &hints, taps);
        ui.add_space(4.0);
    });

    egui::CentralPanel::default().show(root, |ui| {
        // Scrolled like every other list: the rows outgrow a 640x480 screen,
        // and a shorter one (or a larger RETSEND_SCALE) cuts them sooner.
        egui::ScrollArea::vertical()
            .id_salt("settings")
            .show(ui, |ui| {
                for (i, &(at, _)) in state.rows().iter().enumerate() {
                    let (name, value, kind) = &rows[at];
                    if let Some((_, title)) = SECTIONS.iter().find(|(start, _)| *start == at) {
                        if i > 0 {
                            ui.add_space(6.0);
                        }
                        section_header(ui, title);
                    }
                    let selected = state.cursor == i;
                    let desired = egui::vec2(ui.available_width(), theme::ROW_HEIGHT);
                    let (rect, response) = ui.allocate_exact_size(desired, egui::Sense::click());
                    if response.clicked() {
                        taps.push(AppCommand::PickRow(i));
                        taps.push(AppCommand::Confirm);
                    }
                    if selected {
                        super::scroll_to_cursor(ui, ("setting", i), rect);
                        theme::highlight(ui.painter(), rect, theme::accent());
                    }
                    let painter = ui.painter();
                    painter.text(
                        rect.left_center() + egui::vec2(theme::ROW_PAD, 0.0),
                        egui::Align2::LEFT_CENTER,
                        *name,
                        egui::FontId::proportional(theme::ROW_FONT),
                        theme::ink(),
                    );
                    let mut right = rect.right_center() - egui::vec2(theme::ROW_PAD, 0.0);
                    if let Kind::Toggle(on) = kind {
                        let switch = egui::Rect::from_min_max(
                            right - egui::vec2(SWITCH_SIZE.x, SWITCH_SIZE.y / 2.0),
                            right + egui::vec2(0.0, SWITCH_SIZE.y / 2.0),
                        );
                        paint_switch(painter, switch, *on);
                        right.x -= SWITCH_SIZE.x + SWITCH_GAP;
                    }
                    let stepper = matches!(kind, Kind::Stepper(..));
                    let chevron = if selected {
                        theme::accent()
                    } else {
                        theme::dim()
                    };
                    if stepper {
                        icons::paint(
                            painter,
                            Icon::ChevronRight,
                            right - egui::vec2(CHEVRON_SIZE / 2.0, 0.0),
                            CHEVRON_SIZE,
                            chevron,
                        );
                        right.x -= CHEVRON_SIZE + CHEVRON_GAP;
                    }
                    let text = painter.text(
                        right,
                        egui::Align2::RIGHT_CENTER,
                        value,
                        egui::FontId::proportional(theme::DETAIL_FONT),
                        theme::dim(),
                    );
                    if stepper {
                        icons::paint(
                            painter,
                            Icon::ChevronLeft,
                            text.left_center() - egui::vec2(CHEVRON_GAP + CHEVRON_SIZE / 2.0, 0.0),
                            CHEVRON_SIZE,
                            chevron,
                        );
                    }
                }
            });
    });
}

/// A dim caption with a rule under it, grouping the rows below.
pub(super) fn section_header(ui: &mut egui::Ui, title: &str) {
    let (rect, _) = ui.allocate_exact_size(
        egui::vec2(ui.available_width(), HEADER_H),
        egui::Sense::hover(),
    );
    let painter = ui.painter();
    painter.text(
        rect.left_center() + egui::vec2(theme::ROW_PAD, 0.0),
        egui::Align2::LEFT_CENTER,
        title,
        egui::FontId::proportional(theme::DETAIL_FONT),
        theme::dim(),
    );
    painter.hline(
        rect.x_range(),
        rect.bottom() - 1.0,
        egui::Stroke::new(1.0, theme::border()),
    );
}

/// A pill with a knob at one end: lit in the accent when on.
fn paint_switch(painter: &egui::Painter, rect: egui::Rect, on: bool) {
    let radius = rect.height() / 2.0;
    let (fill, knob) = if on {
        (theme::accent(), theme::bg())
    } else {
        (theme::surface(), theme::dim())
    };
    painter.rect(
        rect,
        radius,
        fill,
        egui::Stroke::new(1.0, if on { theme::accent() } else { theme::border() }),
        egui::StrokeKind::Inside,
    );
    let knob_r = radius - KNOB_INSET;
    let x = if on {
        rect.right() - radius
    } else {
        rect.left() + radius
    };
    painter.circle_filled(egui::pos2(x, rect.center().y), knob_r, knob);
}

fn port_label(configured: u16, actual: u16, dirty: bool) -> String {
    if dirty {
        format!("{configured} (applies on close)")
    } else if configured == actual {
        actual.to_string()
    } else {
        format!("{actual} ({configured} was busy)")
    }
}

/// The factor as a percentage of the fit the panel itself asked for. 100% is
/// that fit, which the row spells out — a bare number says nothing about what
/// it is a percentage of.
fn scale_label(scale: f32) -> String {
    let percent = (scale * 100.0).round() as i32;
    if percent == 100 {
        "100% — fits this screen".to_string()
    } else {
        format!("{percent}%")
    }
}

/// "on" with nothing detected has to say so: that is the desktop case, where the
/// setting is live but no console folder exists to route into.
fn auto_routes_value(on: bool, folders: usize) -> String {
    match (on, folders) {
        (false, _) => "off".to_string(),
        (true, 0) => "no console folders found".to_string(),
        (true, n) => format!("ROMs go into {n} console folders"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auto_routes_value_spells_out_each_state() {
        assert_eq!(auto_routes_value(false, 12), "off");
        assert_eq!(auto_routes_value(true, 0), "no console folders found");
        assert_eq!(
            auto_routes_value(true, 12),
            "ROMs go into 12 console folders"
        );
    }

    #[test]
    fn sections_sit_on_rows_that_exist() {
        for (at, _) in SECTIONS {
            assert!(at < crate::overlay::settings::ROW_COUNT);
        }
    }
}
