//! The About screen: the wordmark hero over a small block of build metadata
//! (version, build date, commit) and the project URL. Read-only — B backs out
//! to Settings. All values are compile-time constants (`CARGO_PKG_*` plus the
//! `RETSEND_*` vars stamped by `build.rs`), so nothing is threaded in.

use super::{theme, wordmark};
use egui_sdl2::egui;

pub fn render(root: &mut egui::Ui) {
    // Otherwise-decorative header so About carries the shared top panel like
    // every other base screen; see [`super::TOP_PANEL_ID`] for why every screen
    // must draw one top and one bottom panel under the same ids.
    egui::Panel::top(super::TOP_PANEL_ID).show(root, |ui| {
        ui.add_space(6.0);
        ui.label(
            egui::RichText::new("About")
                .size(theme::ROW_FONT + 2.0)
                .strong(),
        );
        ui.add_space(6.0);
    });

    egui::Panel::bottom(super::BOTTOM_PANEL_ID).show(root, |ui| {
        ui.add_space(4.0);
        super::home::hint_bar(ui, &[("B", "Back")]);
        ui.add_space(4.0);
    });

    egui::CentralPanel::default().show(root, |ui| {
        const HERO_H: f32 = 200.0; // wordmark + gaps + info block, roughly
        let top = ((ui.available_height() - HERO_H) / 2.0).max(8.0);
        ui.vertical_centered(|ui| {
            ui.add_space(top);
            let (_, rect) = ui.allocate_space(wordmark::measure(ui, wordmark::HERO_SIZE));
            wordmark::paint(ui, rect.center(), wordmark::HERO_SIZE, 1.0);
            ui.add_space(6.0);
            ui.label(
                egui::RichText::new(env!("CARGO_PKG_DESCRIPTION"))
                    .size(theme::DETAIL_FONT)
                    .color(theme::DIM),
            );
            ui.add_space(20.0);

            info_row(ui, "Version", env!("CARGO_PKG_VERSION"));
            info_row(ui, "Built", env!("RETSEND_BUILD_DATE"));
            info_row(ui, "Commit", env!("RETSEND_GIT_COMMIT"));
            info_row(ui, "Project", env!("CARGO_PKG_REPOSITORY"));
        });
    });
}

/// A `label   value` pair as one galley — label dim, value accented — so the
/// enclosing `vertical_centered` centers the whole pair as a unit.
fn info_row(ui: &mut egui::Ui, label: &str, value: &str) {
    let font = egui::FontId::proportional(theme::DETAIL_FONT);
    let mut job = egui::text::LayoutJob::default();
    job.append(
        &format!("{label}   "),
        0.0,
        egui::TextFormat {
            font_id: font.clone(),
            color: theme::DIM,
            ..Default::default()
        },
    );
    job.append(
        value,
        0.0,
        egui::TextFormat {
            font_id: font,
            color: theme::ACCENT,
            ..Default::default()
        },
    );
    ui.label(job);
    ui.add_space(4.0);
}
