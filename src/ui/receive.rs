//! The Receive tab: the branded landing screen. The two-tone wordmark hero,
//! who we are on the network (IP + Wi-Fi), and a "ready / no network" status.
//! Incoming requests still arrive as the Prompt modal on top of this.

use super::{theme, wordmark};
use egui_sdl2::egui;

/// Everything the Receive renderer needs, snapshotted by `AppUi::update`.
pub struct ReceiveData {
    pub alias: String,
    /// Announced scheme, uppercased ("HTTPS"/"HTTP").
    pub scheme: String,
    /// The bound TCP port senders dial.
    pub port: u16,
    /// Our LAN address; None means offline — the online/no-network switch.
    pub ip: Option<String>,
    /// Connected Wi-Fi SSID if known (best-effort; None on ethernet/desktop).
    pub ssid: Option<String>,
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

            let dim = |ui: &mut egui::Ui, text: String| {
                ui.label(
                    egui::RichText::new(text)
                        .size(theme::DETAIL_FONT)
                        .color(theme::DIM),
                );
            };

            match &data.ip {
                Some(ip) => {
                    ui.label(
                        egui::RichText::new(format!("Ready to receive as {}", data.alias))
                            .size(theme::ROW_FONT)
                            .strong(),
                    );
                    ui.add_space(4.0);
                    dim(ui, format!("{} · {}:{}", data.scheme, ip, data.port));
                    if let Some(ssid) = &data.ssid {
                        dim(ui, format!("Wi-Fi · {ssid}"));
                    }
                    ui.add_space(12.0);
                    dim(ui, "Waiting for a sender…".to_string());
                    if data.quick_save {
                        ui.add_space(4.0);
                        ui.label(
                            egui::RichText::new("Quick save on — incoming files auto-accepted")
                                .size(theme::DETAIL_FONT)
                                .color(theme::ACCENT),
                        );
                    }
                }
                None => {
                    ui.label(
                        egui::RichText::new("No network connection")
                            .size(theme::ROW_FONT)
                            .strong()
                            .color(theme::DANGER),
                    );
                    ui.add_space(4.0);
                    match &data.ssid {
                        // Associated but no address yet — a DHCP/link hiccup.
                        Some(ssid) => dim(ui, format!("Wi-Fi · {ssid} · waiting for an address")),
                        None => dim(ui, "Not connected to Wi-Fi".to_string()),
                    }
                    ui.add_space(12.0);
                    dim(
                        ui,
                        format!("Connect to a network so senders can reach {}", data.alias),
                    );
                }
            }
        });
    });
}
