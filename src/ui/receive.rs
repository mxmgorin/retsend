//! The Receive tab: the branded landing screen. The brand mark hero, its
//! orbit turning while the device waits, the device's name and a one-line
//! status under it, and the facts a sender might need (address, Wi-Fi, where
//! files land) as chips along the bottom, out of the way.
//! Incoming requests still arrive as the Prompt modal on top of this.

use super::chips::{self, Chip, Rows};
use super::icons::Icon;
use super::{mark, theme};
use crate::app::AppCommand;
use egui_sdl2::egui;
use std::time::Duration;

/// Everything the Receive renderer needs, snapshotted before the frame.
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
    /// Where accepted files land, before any save route redirects them.
    pub save_dir: String,
    /// Quick-save (auto-accept) is on — shown as a chip of its own.
    pub quick_save: bool,
    /// The orbit turns while waiting, a frame every this long; `None` holds
    /// it still. See [`mark::spin_frame`].
    pub spin_frame: Option<Duration>,
}

/// How far an offline mark is pulled toward the plate.
const OFFLINE_DIM: f32 = 0.6;
/// Room a path gets on a chip.
const CHIP_PATH_CHARS: usize = 36;

pub fn render(root: &mut egui::Ui, data: &ReceiveData, taps: &mut Vec<AppCommand>) {
    egui::Panel::bottom(super::BOTTOM_PANEL_ID).show(root, |ui| {
        ui.add_space(4.0);
        // "Refresh" is the radar's word; here the button re-announces us.
        let mut hints: Vec<super::home::Hint> = vec![
            ("← →", "Tabs", None),
            ("Select", "Announce", Some(AppCommand::ReAnnounce)),
        ];
        if crate::app::back_quits() {
            hints.push(("B", "Quit", Some(AppCommand::Back)));
        }
        super::home::hint_bar(ui, &hints, taps);
        ui.add_space(4.0);
    });

    egui::CentralPanel::default().show(root, |ui| {
        let online = data.ip.is_some();
        let chips = chips(data);
        let rows = Rows::pack(ui, &chips, ui.available_width());
        // Still when offline: nothing is being waited for.
        let spin = mark::turning(ui, data.spin_frame.filter(|_| online));
        let (name, name_color, status) = if online {
            (
                data.alias.as_str(),
                theme::ink(),
                "Waiting for a sender…".to_string(),
            )
        } else {
            (
                "No network",
                theme::danger(),
                format!("Connect to Wi-Fi so senders can reach {}", data.alias),
            )
        };
        let dim = if online { 0.0 } else { OFFLINE_DIM };
        chips::hero(ui, rows.height(), spin, dim, name, name_color, &status);
        rows.paint(ui);
    });
}

/// The facts worth a chip, in reading order.
fn chips(data: &ReceiveData) -> Vec<Chip> {
    let mut chips = Vec::new();
    match (&data.ip, &data.ssid) {
        (Some(ip), ssid) => {
            let secure = data.scheme.eq_ignore_ascii_case("https");
            chips.push(Chip {
                icon: if secure { Icon::Lock } else { Icon::LockOpen },
                text: format!("{ip}:{}", data.port),
                color: if secure {
                    theme::dim()
                } else {
                    theme::danger()
                },
            });
            if let Some(ssid) = ssid {
                chips.push(Chip {
                    icon: Icon::Wifi,
                    text: ssid.clone(),
                    color: theme::dim(),
                });
            }
            chips.push(Chip {
                icon: Icon::Folder,
                text: super::truncate_middle(&data.save_dir, CHIP_PATH_CHARS),
                color: theme::dim(),
            });
            if data.quick_save {
                chips.push(Chip {
                    icon: Icon::Check,
                    text: "Quick save".to_string(),
                    color: theme::accent(),
                });
            }
        }
        // Associated but no address yet — a DHCP/link hiccup.
        (None, Some(ssid)) => chips.push(Chip {
            icon: Icon::Wifi,
            text: format!("{ssid} · waiting for an address"),
            color: theme::danger(),
        }),
        (None, None) => chips.push(Chip {
            icon: Icon::Wifi,
            text: "Not connected".to_string(),
            color: theme::danger(),
        }),
    }
    chips
}
