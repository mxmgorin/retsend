//! The routes editor: one row per configured `ext → folder` route plus a
//! trailing "add route" row, then the auto routes as a read-only list. A on the
//! add row starts the add flow (type an extension, then pick a folder); A on a
//! route removes it; B goes back.

use super::theme;
use crate::overlay::routes::RouteCursor;
use egui_sdl2::egui;

pub struct RoutesData {
    /// `(extension, folder)` pairs, sorted (config order).
    pub rows: Vec<(String, String)>,
    pub cursor: RouteCursor,
    /// Detected `(extension, folder)` pairs, listed below the editable ones.
    pub auto_rows: Vec<(String, String)>,
    /// The `auto_routes` setting, to tell "found nothing" from "switched off".
    pub auto_on: bool,
}

pub fn render(root: &mut egui::Ui, data: &RoutesData) {
    egui::Panel::top(super::TOP_PANEL_ID).show(root, |ui| {
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

    egui::Panel::bottom(super::BOTTOM_PANEL_ID).show(root, |ui| {
        ui.add_space(4.0);
        let mut hints = vec![("B", "Back")];
        if let Some(action) = confirm_hint(data.cursor) {
            hints.push(("A", action));
        }
        super::home::hint_bar(ui, &hints);
        ui.add_space(4.0);
    });

    egui::CentralPanel::default().show(root, |ui| {
        egui::ScrollArea::vertical().show(ui, |ui| {
            for (i, (ext, folder)) in data.rows.iter().enumerate() {
                let selected = data.cursor == RouteCursor::Route(i);
                let resp = route_row(ui, ext, folder, selected, false);
                if selected {
                    resp.scroll_to_me(None);
                }
            }
            let on_add = data.cursor == RouteCursor::Add;
            let resp = add_row(ui, on_add);
            if on_add {
                resp.scroll_to_me(None);
            }
            if data.auto_on {
                ui.add_space(10.0);
                ui.label(
                    egui::RichText::new(auto_header(data.auto_rows.len()))
                        .size(theme::DETAIL_FONT)
                        .color(theme::DIM),
                );
                for (i, (ext, folder)) in data.auto_rows.iter().enumerate() {
                    let selected = data.cursor == RouteCursor::Auto(i);
                    let resp = route_row(ui, ext, folder, selected, true);
                    if selected {
                        resp.scroll_to_me(None);
                    }
                }
            }
        });
    });
}

/// The A hint for the row under the cursor; an auto route has no action.
fn confirm_hint(cursor: RouteCursor) -> Option<&'static str> {
    match cursor {
        RouteCursor::Route(_) => Some("Remove"),
        RouteCursor::Add => Some("New route"),
        RouteCursor::Auto(_) => None,
    }
}

/// One `ext → folder` line. `dim` marks an auto route: it has nothing to edit,
/// so its extension is muted like the folder already is.
fn route_row(
    ui: &mut egui::Ui,
    ext: &str,
    folder: &str,
    selected: bool,
    dim: bool,
) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(
        egui::vec2(ui.available_width(), theme::ROW_HEIGHT),
        egui::Sense::hover(),
    );
    if selected {
        highlight(ui, rect);
    }
    let ext_color = if dim {
        theme::DIM
    } else {
        ui.visuals().text_color()
    };
    let painter = ui.painter();
    let padding = 10.0;
    painter.text(
        rect.left_center() + egui::vec2(padding, 0.0),
        egui::Align2::LEFT_CENTER,
        format!(".{ext}"),
        egui::FontId::proportional(theme::ROW_FONT),
        ext_color,
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

/// Heading for the auto-routes list. An empty list is the interesting case: the
/// setting is on, so the reason for the silence belongs on screen.
fn auto_header(rows: usize) -> String {
    match rows {
        0 => "Auto save routes — no console folders in the save folder".to_string(),
        n => format!("Auto save routes ({n})"),
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auto_header_explains_an_empty_list() {
        assert_eq!(
            auto_header(0),
            "Auto save routes — no console folders in the save folder"
        );
        assert_eq!(auto_header(12), "Auto save routes (12)");
    }
}
