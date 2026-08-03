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
            "Existing files",
            if config.transfer.overwrite {
                "replace".into()
            } else {
                "keep both — save as name (1)".into()
            },
            "A Toggle",
        ),
        (
            "Auto save routes",
            auto_routes_value(config.transfer.auto_routes, state.auto_route_count),
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
            "A Open",
        ),
    ];

    // No title panel: the tab bar already shows "⚙ Settings" as the active
    // tab, and an extra top panel here (absent on Send/Receive) would shift
    // egui's panel-id sequence and flag the footer as a changed id.
    egui::Panel::bottom(super::BOTTOM_PANEL_ID).show(root, |ui| {
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

/// "on" with nothing detected has to say so: that is the desktop case, where the
/// setting is live but no console folder exists to route into.
fn auto_routes_value(on: bool, folders: usize) -> String {
    match (on, folders) {
        (false, _) => "off".to_string(),
        (true, 0) => "on — no console folders found".to_string(),
        (true, n) => format!("on — save ROMs into {n} console folders"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auto_routes_value_spells_out_each_state() {
        assert_eq!(auto_routes_value(false, 12), "off");
        assert_eq!(auto_routes_value(true, 0), "on — no console folders found");
        assert_eq!(
            auto_routes_value(true, 12),
            "on — save ROMs into 12 console folders"
        );
    }
}
