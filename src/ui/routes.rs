//! The routes editor: one row per configured `ext → folder` route plus a
//! trailing "add route" row. A on the add row starts the add flow (type an
//! extension, then pick a folder); A on a route removes it; B goes back.

use super::theme;
use egui_sdl2::egui;

pub struct RoutesData {
    /// `(extension, folder)` pairs, sorted (config order).
    pub rows: Vec<(String, String)>,
    /// 0..=rows.len(); `== rows.len()` is the add row.
    pub cursor: usize,
}

pub fn render(root: &mut egui::Ui, data: &RoutesData) {
    let on_add = data.cursor >= data.rows.len();

    egui::Panel::top("routes_header").show(root, |ui| {
        ui.add_space(6.0);
        ui.label(
            egui::RichText::new("Save routes")
                .size(theme::ROW_FONT + 2.0)
                .strong(),
        );
        ui.label(
            egui::RichText::new("Received files go to a folder by extension")
                .size(theme::DETAIL_FONT)
                .color(theme::DIM),
        );
        ui.add_space(6.0);
    });

    egui::Panel::bottom("routes_footer").show(root, |ui| {
        ui.add_space(4.0);
        let action = if on_add { "New route" } else { "Remove" };
        super::home::hint_bar(ui, &[("A", action), ("B", "Back")]);
        ui.add_space(4.0);
    });

    egui::CentralPanel::default().show(root, |ui| {
        egui::ScrollArea::vertical().show(ui, |ui| {
            for (i, (ext, folder)) in data.rows.iter().enumerate() {
                let resp = route_row(ui, ext, folder, data.cursor == i);
                if data.cursor == i {
                    resp.scroll_to_me(None);
                }
            }
            let resp = add_row(ui, on_add);
            if on_add {
                resp.scroll_to_me(None);
            }
        });
    });
}

fn route_row(ui: &mut egui::Ui, ext: &str, folder: &str, selected: bool) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(
        egui::vec2(ui.available_width(), theme::ROW_HEIGHT),
        egui::Sense::hover(),
    );
    if selected {
        highlight(ui, rect);
    }
    let painter = ui.painter();
    let padding = 10.0;
    painter.text(
        rect.left_center() + egui::vec2(padding, 0.0),
        egui::Align2::LEFT_CENTER,
        format!(".{ext}"),
        egui::FontId::proportional(theme::ROW_FONT),
        ui.visuals().text_color(),
    );
    painter.text(
        rect.right_center() - egui::vec2(padding, 0.0),
        egui::Align2::RIGHT_CENTER,
        folder,
        egui::FontId::proportional(theme::DETAIL_FONT),
        theme::DIM,
    );
    response
}

fn add_row(ui: &mut egui::Ui, selected: bool) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(
        egui::vec2(ui.available_width(), theme::ROW_HEIGHT),
        egui::Sense::hover(),
    );
    if selected {
        highlight(ui, rect);
    }
    ui.painter().text(
        rect.left_center() + egui::vec2(10.0, 0.0),
        egui::Align2::LEFT_CENTER,
        "+ Add route",
        egui::FontId::proportional(theme::ROW_FONT),
        theme::ACCENT,
    );
    response
}

fn highlight(ui: &egui::Ui, rect: egui::Rect) {
    ui.painter().rect(
        rect,
        6.0,
        theme::ACCENT.linear_multiply(0.30),
        egui::Stroke::new(1.0, theme::ACCENT),
        egui::StrokeKind::Inside,
    );
}
