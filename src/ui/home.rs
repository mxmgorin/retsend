//! The radar screen: our identity up top, nearby devices in the middle,
//! button hints at the bottom.

use super::theme;
use egui_sdl2::egui;

/// A display-ready radar row. `AppUi::update` builds these (from the peer
/// registry once discovery exists), keeping this renderer decoupled from the
/// net layer.
pub struct PeerRow {
    pub alias: String,
    /// Shown under the alias, e.g. "Pixel · 192.168.1.23".
    pub detail: String,
    /// Transport badge on the right: "HTTP" / "HTTPS".
    pub proto: String,
}

/// Everything the renderer needs, snapshotted by `AppUi::update` outside the
/// egui closure (shared-state locks must not be held while egui borrows `self`).
pub struct HomeData {
    pub alias: String,
    /// e.g. "HTTP · 192.168.1.42:53317"
    pub endpoint: String,
    pub peers: Vec<PeerRow>,
    pub cursor: Option<usize>,
}

pub fn render(root: &mut egui::Ui, data: &HomeData) {
    egui::Panel::top("home_header").show_inside(root, |ui| {
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new(&data.alias)
                    .size(theme::ROW_FONT + 2.0)
                    .strong(),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(
                    egui::RichText::new(&data.endpoint)
                        .size(theme::DETAIL_FONT)
                        .color(theme::DIM),
                );
            });
        });
        ui.add_space(6.0);
    });

    egui::Panel::bottom("home_footer").show_inside(root, |ui| {
        ui.add_space(4.0);
        hint_bar(
            ui,
            &[("A", "Send"), ("Select", "Refresh"), ("Start", "Settings")],
        );
        ui.add_space(4.0);
    });

    egui::CentralPanel::default().show_inside(root, |ui| {
        if data.peers.is_empty() {
            ui.centered_and_justified(|ui| {
                ui.label(
                    egui::RichText::new(
                        "Searching for devices…\n\nOpen LocalSend on your phone or PC\non the same network.",
                    )
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

/// `[Btn] Action · [Btn] Action` bar shared by the screens.
pub fn hint_bar(ui: &mut egui::Ui, hints: &[(&str, &str)]) {
    ui.horizontal(|ui| {
        for (i, (button, action)) in hints.iter().enumerate() {
            if i > 0 {
                ui.label(egui::RichText::new("·").color(theme::DIM));
            }
            ui.label(
                egui::RichText::new(*button)
                    .size(theme::DETAIL_FONT)
                    .strong()
                    .color(theme::ACCENT),
            );
            ui.label(
                egui::RichText::new(*action)
                    .size(theme::DETAIL_FONT)
                    .color(theme::DIM),
            );
        }
    });
}
