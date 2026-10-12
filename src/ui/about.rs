//! The About screen: the brand mark and wordmark over a small block of build
//! metadata (version, build date, commit), the project URL and author.
//! Read-only — B backs out to Settings. All values are compile-time constants
//! (`CARGO_PKG_*` plus the `RETSEND_*` vars stamped by `build.rs`), so nothing
//! is threaded in.

use super::{mark, theme};
use crate::app::AppCommand;
use egui_sdl2::egui;

/// The wordmark's type size under the hero.
const WORDMARK_SIZE: f32 = 30.0;

pub fn render(root: &mut egui::Ui, taps: &mut Vec<AppCommand>) {
    // A decorative header: every base screen draws one top and one bottom
    // panel under the shared ids, see [`super::TOP_PANEL_ID`].
    egui::Panel::top(super::TOP_PANEL_ID).show(root, |ui| {
        ui.add_space(6.0);
        ui.label(
            egui::RichText::new("About")
                .size(theme::TITLE_FONT)
                .strong(),
        );
        ui.add_space(6.0);
    });

    egui::Panel::bottom(super::BOTTOM_PANEL_ID).show(root, |ui| {
        ui.add_space(4.0);
        super::home::hint_bar(ui, &[("B", "Back", Some(AppCommand::Back))], taps);
        ui.add_space(4.0);
    });

    egui::CentralPanel::default().show(root, |ui| {
        const HERO_H: f32 = mark::HERO_SIZE + 194.0; // mark + wordmark + gaps + info block, roughly
        let top = ((ui.available_height() - HERO_H) / 2.0).max(8.0);
        ui.vertical_centered(|ui| {
            ui.add_space(top);
            let (_, rect) = ui.allocate_space(mark::measure(mark::HERO_SIZE));
            mark::paint(ui.painter(), rect.center(), mark::HERO_SIZE, 0.0, 0.0);
            ui.add_space(8.0);
            let wordmark = mark::wordmark(ui, WORDMARK_SIZE);
            let (_, rect) = ui.allocate_space(wordmark.size());
            ui.painter().galley(rect.min, wordmark, theme::INK);
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
            info_row(ui, "Author", env!("CARGO_PKG_AUTHORS"));
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
