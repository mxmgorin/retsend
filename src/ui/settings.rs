//! Settings screen renderer: name/value rows with per-row edit hints.

use super::theme;
use crate::config::AppConfig;
use crate::overlay::settings::Settings;
use egui_sdl2::egui;

pub fn render(root: &mut egui::Ui, state: &Settings, config: &AppConfig, actual_port: u16) {
    let rows: [(&str, String, &str); crate::overlay::settings::ROW_COUNT] = [
        ("Alias", config.device.alias.clone(), "A Edit"),
        ("Save to", config.transfer.save_dir.clone(), "A Choose"),
        (
            "Port",
            port_label(config.network.port, actual_port, state.port_dirty),
            "← → Adjust",
        ),
        (
            "Quick save",
            if config.transfer.auto_accept {
                "on — accept without asking".into()
            } else {
                "off".into()
            },
            "A Toggle",
        ),
        (
            "Save routes",
            match config.transfer.routes.len() {
                0 => "none".into(),
                1 => "1 extension".into(),
                n => format!("{n} extensions"),
            },
            "A Edit",
        ),
        (
            "About",
            format!("retsend {}", env!("CARGO_PKG_VERSION")),
            "",
        ),
    ];

    // No title panel: the tab bar already shows "⚙ Settings" as the active
    // tab, and an extra top panel here (absent on Send/Receive) would shift
    // egui's panel-id sequence and flag the footer as a changed id.
    egui::Panel::bottom("tab_footer").show(root, |ui| {
        ui.add_space(4.0);
        super::home::hint_bar(ui, &[("L1/R1", "Tabs")]);
        ui.add_space(4.0);
    });

    egui::CentralPanel::default().show(root, |ui| {
        for (i, (name, value, hint)) in rows.iter().enumerate() {
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
                rect.left_top() + egui::vec2(padding, 7.0),
                egui::Align2::LEFT_TOP,
                *name,
                egui::FontId::proportional(theme::ROW_FONT),
                ui.visuals().text_color(),
            );
            if selected && !hint.is_empty() {
                painter.text(
                    rect.left_bottom() + egui::vec2(padding, -6.0),
                    egui::Align2::LEFT_BOTTOM,
                    *hint,
                    egui::FontId::proportional(theme::DETAIL_FONT - 1.0),
                    theme::ACCENT,
                );
            }
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

fn port_label(configured: u16, actual: u16, dirty: bool) -> String {
    if dirty {
        format!("{configured} (applies on close)")
    } else if configured == actual {
        actual.to_string()
    } else {
        format!("{actual} ({configured} was busy)")
    }
}
