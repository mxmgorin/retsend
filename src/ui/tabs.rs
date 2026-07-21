//! The top tab bar shared by the three tab screens: `[↑ Send] [↓ Receive]
//! [⚙ Settings]`, spread evenly across the bar with the active tab in the
//! accent pill. Hidden during the Browser / Transfer / Prompt / Osk takeovers
//! (`crate::ui::mod` decides).

use super::theme;
use crate::overlay::tabs::Tab;
use egui_sdl2::egui;

/// Send/receive read as upload/download arrows; the gear is the usual settings
/// glyph. All three resolve through the Proportional fallback chain (arrows in
/// Ubuntu, the gear in NotoEmoji).
const TABS: [(Tab, &str, &str); 3] = [
    (Tab::Send, "↑", "Send"),
    (Tab::Receive, "↓", "Receive"),
    (Tab::Settings, "⚙", "Settings"),
];

/// Inner padding of a pill (text → pill edge).
const PAD: egui::Vec2 = egui::vec2(12.0, 4.0);

pub fn render_bar(root: &mut egui::Ui, active: Tab) {
    egui::Panel::top("tab_bar").show_inside(root, |ui| {
        ui.add_space(6.0);

        // Painted directly over one reserved row (no nested layout, no
        // per-tab interactive widgets): egui runs a second pass on tab
        // switches, and widget ids shifting between passes paint a red line.
        let font = egui::FontId::proportional(theme::ROW_FONT);
        let items: Vec<_> = TABS
            .iter()
            .map(|(tab, icon, label)| {
                let is_active = *tab == active;
                let color = if is_active { INK } else { theme::DIM };
                let galley =
                    ui.painter()
                        .layout_no_wrap(format!("{icon}  {label}"), font.clone(), color);
                (galley, color, is_active)
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

        for (i, (galley, color, is_active)) in items.into_iter().enumerate() {
            // Center each pill in its equal slice of the bar.
            let center = egui::pos2(rect.left() + slot_w * (i as f32 + 0.5), rect.center().y);
            let pill = egui::Rect::from_center_size(center, galley.size() + PAD * 2.0);
            if is_active {
                ui.painter().rect(
                    pill,
                    6.0,
                    theme::ACCENT.linear_multiply(0.30),
                    egui::Stroke::new(1.0, theme::ACCENT),
                    egui::StrokeKind::Inside,
                );
            }
            ui.painter().galley(pill.min + PAD, galley, color);
        }
        ui.add_space(6.0);
    });
}

/// Active-tab text, matching the wordmark ink.
const INK: egui::Color32 = egui::Color32::from_rgb(0xec, 0xec, 0xea);
