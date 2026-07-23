//! The Send tab: nearby devices in the middle, button hints at the bottom.
//! The tab bar above it is drawn by `super::tabs`; our identity moved to the
//! Receive tab.

use super::{theme, wordmark};
use egui_sdl2::egui;

/// A display-ready radar row. `AppUi::update` builds these from the peer
/// registry, keeping this renderer decoupled from the net layer.
pub struct PeerRow {
    pub alias: String,
    /// Shown under the alias, e.g. "Pixel · 192.168.1.23".
    pub detail: String,
    /// Transport badge on the right: "HTTP" / "HTTPS".
    pub proto: String,
}

/// Everything the Send renderer needs, snapshotted by `AppUi::update` outside
/// the egui closure (shared-state locks must not be held while egui borrows
/// `self`).
pub struct HomeData {
    pub peers: Vec<PeerRow>,
    pub cursor: Option<usize>,
}

pub fn render(root: &mut egui::Ui, data: &HomeData) {
    egui::Panel::bottom("tab_footer").show(root, |ui| {
        ui.add_space(4.0);
        hint_bar(
            ui,
            &[("A", "Send"), ("L1/R1", "Tabs"), ("Select", "Refresh")],
        );
        ui.add_space(4.0);
    });

    egui::CentralPanel::default().show(root, |ui| {
        if data.peers.is_empty() {
            // Same branded hero as the Receive tab, with a discovery hint.
            const HERO_H: f32 = 130.0; // wordmark + gap + hint, roughly
            let top = ((ui.available_height() - HERO_H) / 2.0).max(8.0);
            ui.vertical_centered(|ui| {
                ui.add_space(top);
                let (_, rect) = ui.allocate_space(wordmark::measure(ui, wordmark::HERO_SIZE));
                wordmark::paint(ui, rect.center(), wordmark::HERO_SIZE, 1.0);
                ui.add_space(24.0);
                ui.label(
                    egui::RichText::new("Open LocalSend on your phone or PC\non the same network.")
                        .size(theme::ROW_FONT)
                        .color(theme::DIM),
                );
            });
            return;
        }

        egui::ScrollArea::vertical().show(ui, |ui| {
            for (i, peer) in data.peers.iter().enumerate() {
                let selected = data.cursor == Some(i);
                let row = peer_row(ui, peer, selected);
                if selected {
                    row.scroll_to_me(None);
                }
            }
        });
    });
}

fn peer_row(ui: &mut egui::Ui, peer: &PeerRow, selected: bool) -> egui::Response {
    let desired = egui::vec2(ui.available_width(), theme::ROW_HEIGHT);
    let (rect, response) = ui.allocate_exact_size(desired, egui::Sense::hover());
    if selected {
        ui.painter().rect(
            rect,
            6.0,
            theme::ACCENT.linear_multiply(0.30),
            egui::Stroke::new(1.0, theme::ACCENT),
            egui::StrokeKind::Inside,
        );
    }
    let padding = 10.0;
    let painter = ui.painter();
    painter.text(
        rect.left_top() + egui::vec2(padding, 7.0),
        egui::Align2::LEFT_TOP,
        &peer.alias,
        egui::FontId::proportional(theme::ROW_FONT),
        ui.visuals().text_color(),
    );
    painter.text(
        rect.left_bottom() + egui::vec2(padding, -7.0),
        egui::Align2::LEFT_BOTTOM,
        &peer.detail,
        egui::FontId::proportional(theme::DETAIL_FONT),
        theme::DIM,
    );
    // Transport badge: peers announcing https need the HTTPS milestone before
    // we can send to them; make that visible early.
    painter.text(
        rect.right_center() - egui::vec2(padding, 0.0),
        egui::Align2::RIGHT_CENTER,
        &peer.proto,
        egui::FontId::proportional(theme::DETAIL_FONT),
        theme::DIM,
    );
    response
}

/// `[Btn] Action` hints spread evenly across the width — each hint owns an
/// equal slot and sits centered in it, matching the tab bar. Shared by every
/// screen's footer.
pub fn hint_bar(ui: &mut egui::Ui, hints: &[(&str, &str)]) {
    if hints.is_empty() {
        return;
    }
    // Painted directly over one reserved row — no nested layout, no per-slot
    // interactive widgets. The hint count differs per tab, and any widget id
    // shifting between egui's passes paints a red line at the panel edge.
    let galleys: Vec<_> = hints
        .iter()
        .map(|(button, action)| hint_galley(ui, button, action))
        .collect();
    let row_h = galleys.iter().map(|g| g.size().y).fold(0.0_f32, f32::max);
    let full_w = ui.available_width();
    let (_, rect) = ui.allocate_space(egui::vec2(full_w, row_h));
    let slot_w = full_w / hints.len() as f32;
    for (i, galley) in galleys.into_iter().enumerate() {
        let center = egui::pos2(rect.left() + slot_w * (i as f32 + 0.5), rect.center().y);
        ui.painter()
            .galley(center - galley.size() / 2.0, galley, theme::DIM);
    }
}

/// A `button` (accent) + `action` (dim) hint laid out as one galley so it can
/// be centered in its slot as a unit.
fn hint_galley(ui: &egui::Ui, button: &str, action: &str) -> std::sync::Arc<egui::Galley> {
    let font = egui::FontId::proportional(theme::DETAIL_FONT);
    let mut job = egui::text::LayoutJob::default();
    job.append(
        button,
        0.0,
        egui::TextFormat {
            font_id: font.clone(),
            color: theme::ACCENT,
            ..Default::default()
        },
    );
    job.append(
        action,
        6.0,
        egui::TextFormat {
            font_id: font,
            color: theme::DIM,
            ..Default::default()
        },
    );
    ui.ctx().fonts_mut(|f| f.layout_job(job))
}
