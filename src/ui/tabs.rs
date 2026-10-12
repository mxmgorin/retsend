//! The top tab bar shared by the four tab screens: Receive, Send, History and
//! Settings, spread evenly across the bar with the active tab in a pill of its
//! own color. Hidden during the Browser / Transfer / Prompt / Osk takeovers
//! (`crate::ui::mod` decides).

use super::icons::{self, Icon};
use super::theme;
use crate::app::AppCommand;
use crate::overlay::tabs::Tab;
use egui_sdl2::egui;

/// Mirrors `crate::overlay::tabs::ORDER`.
const TABS: [(Tab, Icon, &str); 4] = [
    (Tab::Receive, Icon::Receive, "Receive"),
    (Tab::Send, Icon::Send, "Send"),
    (Tab::History, Icon::History, "History"),
    (Tab::Settings, Icon::Settings, "Settings"),
];

/// Inner padding of a pill (content → pill edge).
const PAD: egui::Vec2 = egui::vec2(12.0, 4.0);
/// Space between a tab's icon and its label.
const ICON_GAP: f32 = 7.0;

/// The pill color of a tab: coral for Send, the accent for the rest.
fn tab_color(tab: Tab) -> egui::Color32 {
    theme::flow_color(tab == Tab::Send)
}

pub fn render_bar(root: &mut egui::Ui, active: Tab, taps: &mut Vec<AppCommand>) {
    egui::Panel::top(super::TOP_PANEL_ID).show(root, |ui| {
        ui.add_space(6.0);

        // Painted over one reserved row, no per-tab widgets: ids shifting
        // between egui's passes on a tab switch paint a red line.
        let font = egui::FontId::proportional(theme::ROW_FONT);
        let items: Vec<_> = TABS
            .iter()
            .map(|(tab, icon, label)| {
                let is_active = *tab == active;
                let color = if is_active { theme::INK } else { theme::DIM };
                let galley = ui
                    .painter()
                    .layout_no_wrap(label.to_string(), font.clone(), color);
                (galley, *icon, color, is_active, tab_color(*tab))
            })
            .collect();

        let row_h = items
            .iter()
            .map(|(g, ..)| g.size().y)
            .fold(0.0_f32, f32::max)
            + PAD.y * 2.0;
        let full_w = ui.available_width();
        let (_, rect) = ui.allocate_space(egui::vec2(full_w, row_h));
        let slot_w = full_w / TABS.len() as f32;

        let tap = super::home::tap_pos(ui);
        for (i, (galley, icon, color, is_active, pill_color)) in items.into_iter().enumerate() {
            let center = egui::pos2(rect.left() + slot_w * (i as f32 + 0.5), rect.center().y);
            let content = egui::vec2(
                theme::ICON_SIZE + ICON_GAP + galley.size().x,
                galley.size().y,
            );
            let pill = egui::Rect::from_center_size(center, content + PAD * 2.0);
            // The whole slice takes the tap, not just the pill — four pills
            // leave gaps a finger falls into.
            let slot = egui::Rect::from_center_size(center, egui::vec2(slot_w, row_h));
            if tap.is_some_and(|pos| slot.contains(pos)) {
                taps.push(AppCommand::PickTab(TABS[i].0));
            }
            if is_active {
                theme::highlight(ui.painter(), pill, pill_color);
            }
            let icon_color = if is_active { pill_color } else { color };
            let left = pill.min + PAD;
            icons::paint(
                ui.painter(),
                icon,
                egui::pos2(left.x + theme::ICON_SIZE / 2.0, center.y),
                theme::ICON_SIZE,
                icon_color,
            );
            ui.painter().galley(
                left + egui::vec2(theme::ICON_SIZE + ICON_GAP, 0.0),
                galley,
                color,
            );
        }
        ui.add_space(6.0);
    });
}
