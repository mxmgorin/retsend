//! The Receive tab: the branded landing screen. The two-tone wordmark hero,
//! who we are on the network, and a "ready / waiting" status. Incoming
//! requests still arrive as the Prompt modal on top of this.

use super::{theme, wordmark};
use egui_sdl2::egui;

/// Everything the Receive renderer needs, snapshotted by `AppUi::update`.
pub struct ReceiveData {
    pub alias: String,
    /// e.g. "HTTPS · 192.168.1.42:53317"
    pub endpoint: String,
    /// Quick-save (auto-accept) is on — shown as a badge under the status.
    pub quick_save: bool,
}

pub fn render(root: &mut egui::Ui, data: &ReceiveData) {
    egui::Panel::bottom("tab_footer").show(root, |ui| {
        ui.add_space(4.0);
        super::home::hint_bar(ui, &[("L1/R1", "Tabs"), ("Select", "Refresh")]);
        ui.add_space(4.0);
    });

    egui::CentralPanel::default().show(root, |ui| {
        const HERO_H: f32 = 190.0; // wordmark + gaps + status block, roughly
        let top = ((ui.available_height() - HERO_H) / 2.0).max(8.0);
        ui.vertical_centered(|ui| {
            ui.add_space(top);
            let (_, rect) = ui.allocate_space(wordmark::measure(ui, wordmark::HERO_SIZE));
            wordmark::paint(ui, rect.center(), wordmark::HERO_SIZE, 1.0);
            ui.add_space(24.0);
            ui.label(
                egui::RichText::new(format!("Ready to receive as {}", data.alias))
                    .size(theme::ROW_FONT)
                    .strong(),
            );
            ui.add_space(4.0);
            ui.label(
                egui::RichText::new(&data.endpoint)
                    .size(theme::DETAIL_FONT)
                    .color(theme::DIM),
            );
            ui.add_space(12.0);
            ui.label(
                egui::RichText::new("Waiting for a sender…")
                    .size(theme::DETAIL_FONT)
                    .color(theme::DIM),
            );
            if data.quick_save {
                ui.add_space(4.0);
                ui.label(
                    egui::RichText::new("Quick save on — incoming files auto-accepted")
                        .size(theme::DETAIL_FONT)
                        .color(theme::ACCENT),
                );
            }
        });
    });
}
