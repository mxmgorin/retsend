//! Settings screen renderer. Read-only in this milestone; the cursor and row
//! layout are final so editing only swaps the value side later.

use super::theme;
use crate::config::AppConfig;
use crate::overlay::settings::Settings;
use egui_sdl2::egui;

pub fn render(root: &mut egui::Ui, state: &Settings, config: &AppConfig, actual_port: u16) {
    let rows: [(&str, String); crate::overlay::settings::ROW_COUNT] = [
        ("Alias", config.device.alias.clone()),
        ("Save to", config.transfer.save_dir.clone()),
        ("Port", port_label(config.network.port, actual_port)),
        (
            "Quick save",
            if config.transfer.auto_accept {
                "on (accept without asking)".into()
            } else {
                "off".into()
            },
        ),
        (
            "About",
            format!("localsend-retro {}", env!("CARGO_PKG_VERSION")),
        ),
    ];

    egui::Panel::top("settings_header").show_inside(root, |ui| {
        ui.add_space(6.0);
        ui.label(
            egui::RichText::new("Settings")
                .size(theme::ROW_FONT + 2.0)
                .strong(),
        );
        ui.add_space(6.0);
    });

    egui::Panel::bottom("settings_footer").show_inside(root, |ui| {
        ui.add_space(4.0);
        super::home::hint_bar(ui, &[("B", "Back")]);
        ui.add_space(4.0);
    });

    egui::CentralPanel::default().show_inside(root, |ui| {
        ui.label(
            egui::RichText::new("Read-only for now — edit config.toml; editing lands soon.")
                .size(theme::DETAIL_FONT)
                .color(theme::DIM),
        );
        ui.add_space(6.0);
        for (i, (name, value)) in rows.iter().enumerate() {
            let selected = state.cursor == i;
            let desired = egui::vec2(ui.available_width(), theme::ROW_HEIGHT);
            let (rect, _) = ui.allocate_exact_size(desired, egui::Sense::hover());
            if selected {
                ui.painter().rect(
                    rect,
                    6.0,
                    theme::ACCENT.linear_multiply(0.30),
                    egui::Stroke::new(1.0, theme::ACCENT),
                    egui::StrokeKind::Inside,
                );
            }
            let painter = ui.painter();
            let padding = 10.0;
            painter.text(
                rect.left_center() + egui::vec2(padding, 0.0),
                egui::Align2::LEFT_CENTER,
                *name,
                egui::FontId::proportional(theme::ROW_FONT),
                ui.visuals().text_color(),
            );
            painter.text(
                rect.right_center() - egui::vec2(padding, 0.0),
                egui::Align2::RIGHT_CENTER,
                value,
                egui::FontId::proportional(theme::DETAIL_FONT),
                theme::DIM,
            );
        }
    });
}

fn port_label(configured: u16, actual: u16) -> String {
    if configured == actual {
        actual.to_string()
    } else {
        format!("{actual} ({configured} was busy)")
    }
}
