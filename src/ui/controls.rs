//! The keys editor: one row per button with the keys it answers to, then
//! "remap all" and "reset all" rows. During a remap each row shows the key it
//! has taken so far.

use super::theme;
use crate::app::AppCommand;
use crate::config::input::{KeyBindings, PadButton};
use crate::overlay::controls::{Capture, ControlsRow, ControlsView, REMAP_ORDER};
use egui_sdl2::egui;

pub struct ControlsData {
    /// `(button label, its keys, whether a running remap has set them)`, in
    /// [`PadButton::ALL`] order.
    pub rows: Vec<(&'static str, String, bool)>,
    /// The row lit: the cursor's, or the button a remap is asking for.
    pub lit: usize,
    pub row: ControlsRow,
    pub capturing: bool,
    /// The wait's countdown, in whole seconds rounded up.
    pub seconds_left: u64,
    /// While a remap runs: the step, from 1, and the label it asks for.
    pub remap: Option<(usize, &'static str)>,
}

impl ControlsData {
    pub fn new(view: &ControlsView, keys: &KeyBindings) -> Self {
        let taken: &[sdl2::keyboard::Keycode] = match view.capture() {
            Capture::Remap(taken) => taken,
            _ => &[],
        };
        let rows = PadButton::ALL
            .iter()
            .map(|&button| {
                let fresh = REMAP_ORDER[..taken.len()]
                    .iter()
                    .position(|&b| b == button)
                    .map(|at| taken[at].name());
                match fresh {
                    Some(name) => (
                        button.label(),
                        crate::event::key_label(&name).to_string(),
                        true,
                    ),
                    None => {
                        let names = crate::event::effective_keys(view.layout, keys, button);
                        (button.label(), keys_label(&names), false)
                    }
                }
            })
            .collect();
        let asking = view.remap_button();
        let lit = asking
            .and_then(|b| PadButton::ALL.iter().position(|&a| a == b))
            .unwrap_or(view.cursor());
        Self {
            rows,
            lit,
            row: view.row(),
            capturing: view.is_capturing(),
            seconds_left: view
                .time_left()
                .map_or(0, |t| t.as_secs() + u64::from(t.subsec_nanos() > 0)),
            remap: asking.map(|b| (taken.len() + 1, b.label())),
        }
    }
}

pub fn render(root: &mut egui::Ui, data: &ControlsData, taps: &mut Vec<AppCommand>) {
    egui::Panel::top(super::TOP_PANEL_ID).show(root, |ui| {
        ui.add_space(6.0);
        ui.label(egui::RichText::new("Keys").size(theme::TITLE_FONT).strong());
        let subtitle = match data.remap {
            Some((step, label)) => format!(
                "Remap {step}/{}: press a key for {label}",
                REMAP_ORDER.len()
            ),
            None => "Keyboard keys per button; a gamepad keeps its buttons".to_string(),
        };
        let color = if data.remap.is_some() {
            theme::accent()
        } else {
            theme::dim()
        };
        ui.label(
            egui::RichText::new(subtitle)
                .size(theme::DETAIL_FONT)
                .color(color),
        );
        ui.add_space(6.0);
    });

    egui::Panel::bottom(super::BOTTOM_PANEL_ID).show(root, |ui| {
        ui.add_space(4.0);
        let back = ("B", "Back", Some(AppCommand::Back));
        let hints: Vec<super::home::Hint> = match (data.capturing, data.row) {
            (true, _) => vec![("B", "Cancel", Some(AppCommand::Back))],
            (false, ControlsRow::Button(_)) => vec![
                back,
                ("A", "Add key", Some(AppCommand::Confirm)),
                ("X", "Reset", Some(AppCommand::Alt)),
            ],
            (false, ControlsRow::RemapAll) => vec![back, ("A", "Start", Some(AppCommand::Confirm))],
            (false, ControlsRow::ResetAll) => {
                vec![back, ("A", "Reset all", Some(AppCommand::Confirm))]
            }
        };
        super::home::hint_bar(ui, &hints, taps);
        ui.add_space(4.0);
    });

    egui::CentralPanel::default().show(root, |ui| {
        egui::ScrollArea::vertical()
            .id_salt("controls")
            .show(ui, |ui| {
                for (i, (label, keys, fresh)) in data.rows.iter().enumerate() {
                    let lit = data.lit == i;
                    let waiting = format!("press a key… {}s", data.seconds_left);
                    let value = if lit && data.capturing {
                        (waiting.as_str(), theme::accent())
                    } else if *fresh {
                        (keys.as_str(), theme::accent())
                    } else {
                        (keys.as_str(), theme::dim())
                    };
                    let resp = row(ui, label, theme::ink(), Some(value), lit);
                    if lit {
                        super::scroll_to_cursor(ui, ("controls", i), resp.rect);
                    }
                    tapped(&resp, i, taps);
                }
                ui.add_space(6.0);
                let tail = [
                    ("Remap all keys", data.rows.len()),
                    ("Reset all keys", data.rows.len() + 1),
                ];
                for (label, index) in tail {
                    let lit = data.lit == index && !data.capturing;
                    let resp = row(ui, label, theme::accent(), None, lit);
                    if lit {
                        super::scroll_to_cursor(ui, ("controls", index), resp.rect);
                    }
                    tapped(&resp, index, taps);
                }
            });
    });
}

fn tapped(resp: &egui::Response, index: usize, taps: &mut Vec<AppCommand>) {
    if resp.clicked() {
        taps.push(AppCommand::PickRow(index));
        taps.push(AppCommand::Confirm);
    }
}

fn row(
    ui: &mut egui::Ui,
    label: &str,
    color: egui::Color32,
    value: Option<(&str, egui::Color32)>,
    selected: bool,
) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(
        egui::vec2(ui.available_width(), theme::ROW_HEIGHT),
        egui::Sense::click(),
    );
    if selected {
        theme::highlight(ui.painter(), rect, theme::accent());
    }
    let painter = ui.painter();
    painter.text(
        rect.left_center() + egui::vec2(theme::ROW_PAD, 0.0),
        egui::Align2::LEFT_CENTER,
        label,
        egui::FontId::proportional(theme::ROW_FONT),
        color,
    );
    if let Some((text, color)) = value {
        painter.text(
            rect.right_center() - egui::vec2(theme::ROW_PAD, 0.0),
            egui::Align2::RIGHT_CENTER,
            text,
            egui::FontId::proportional(theme::DETAIL_FONT),
            color,
        );
    }
    response
}

/// The keys as one line; an unbound button says so.
fn keys_label(names: &[String]) -> String {
    if names.is_empty() {
        return "none".to_string();
    }
    let labels: Vec<&str> = names
        .iter()
        .map(|name| crate::event::key_label(name))
        .collect();
    labels.join(" · ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys_label_joins_and_names_the_empty_case() {
        assert_eq!(keys_label(&[]), "none");
        assert_eq!(
            keys_label(&["Return".into(), "Select".into(), "5".into()]),
            "Enter · OK · 5"
        );
    }
}
