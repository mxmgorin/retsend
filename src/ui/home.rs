//! The Send tab: nearby devices in the middle, button hints at the bottom.
//! The tab bar above it is drawn by `super::tabs`; our identity lives on the
//! Receive tab. Also home of the footer hint bar every screen shares.

use super::chips::{self, Chip, Rows};
use super::icons::{self, Icon};
use super::{mark, theme};
use crate::app::AppCommand;
use egui_sdl2::egui;

/// A footer hint: the button, what it does, and the command a tap on its slot
/// stands for — `None` for the ones naming no single command.
pub type Hint<'a> = (&'a str, &'a str, Option<AppCommand>);

/// A hint slot is one text row tall; taps get a little more to aim at.
const HINT_TAP_PAD: f32 = 6.0;

/// A display-ready radar row, decoupled from the peer registry.
pub struct PeerRow {
    pub alias: String,
    /// Shown under the alias, e.g. "Pixel · 192.168.1.23".
    pub detail: String,
    /// What the peer announced itself as.
    pub icon: Icon,
    /// Announced plain HTTP — badged on the right. Encryption is the norm, so
    /// only its absence is worth the pixels.
    pub insecure: bool,
}

/// Everything the Send renderer needs, snapshotted before the egui closure:
/// shared-state locks must not be held while egui borrows the UI.
pub struct HomeData {
    pub peers: Vec<PeerRow>,
    pub cursor: Option<usize>,
    /// Connected Wi-Fi SSID if known: the network a device must share.
    pub ssid: Option<String>,
    /// The orbit turns while the radar is empty, a frame every this long;
    /// `None` holds it still. See [`mark::spin_frame`].
    pub spin_frame: Option<std::time::Duration>,
}

/// Gutter a row's icon takes before its text.
const ICON_GUTTER: f32 = 30.0;

pub fn render(root: &mut egui::Ui, data: &HomeData, taps: &mut Vec<AppCommand>) {
    egui::Panel::bottom(super::BOTTOM_PANEL_ID).show(root, |ui| {
        ui.add_space(4.0);
        let mut hints: Vec<Hint> = vec![
            ("← →", "Tabs", None),
            ("Select", "Refresh", Some(AppCommand::ReAnnounce)),
            ("X", "Add IP", Some(AppCommand::Alt)),
            ("A", "Choose files", Some(AppCommand::Confirm)),
        ];
        if crate::app::back_quits() {
            hints.push(("B", "Quit", Some(AppCommand::Back)));
        }
        hint_bar(ui, &hints, taps);
        ui.add_space(4.0);
    });

    egui::CentralPanel::default().show(root, |ui| {
        if data.peers.is_empty() {
            // The Receive hero, its orbit sweeping like the radar it is; the
            // network and the add-by-IP escape hatch go on chips.
            let mut facts = Vec::new();
            if let Some(ssid) = &data.ssid {
                facts.push(Chip {
                    icon: Icon::Wifi,
                    text: ssid.clone(),
                    color: theme::DIM,
                });
            }
            facts.push(Chip {
                icon: Icon::Plus,
                text: "Not showing up? X adds a device by IP".to_string(),
                color: theme::DIM,
            });
            let rows = Rows::pack(ui, &facts, ui.available_width());
            let spin = mark::turning(ui, data.spin_frame);
            chips::hero(
                ui,
                rows.height(),
                spin,
                0.0,
                "Looking for devices…",
                theme::INK,
                "Open LocalSend on a phone or PC on the same network",
            );
            rows.paint(ui);
            return;
        }

        egui::ScrollArea::vertical()
            .id_salt("peers")
            .show(ui, |ui| {
                for (i, peer) in data.peers.iter().enumerate() {
                    let selected = data.cursor == Some(i);
                    let row = peer_row(ui, peer, selected);
                    if selected {
                        super::scroll_to_cursor(ui, ("peers", i), row.rect);
                    }
                    if row.clicked() {
                        taps.push(AppCommand::PickRow(i));
                        taps.push(AppCommand::Confirm);
                    }
                }
            });
    });
}

fn peer_row(ui: &mut egui::Ui, peer: &PeerRow, selected: bool) -> egui::Response {
    let desired = egui::vec2(ui.available_width(), theme::ROW_HEIGHT);
    let (rect, response) = ui.allocate_exact_size(desired, egui::Sense::click());
    // Coral: a row here is somewhere to send to.
    let flow = theme::flow_color(true);
    if selected {
        theme::highlight(ui.painter(), rect, flow);
    }
    let painter = ui.painter();
    let icon_color = if selected { flow } else { theme::DIM };
    icons::paint(
        painter,
        peer.icon,
        rect.left_center() + egui::vec2(theme::ROW_PAD + theme::ICON_SIZE / 2.0, 0.0),
        theme::ICON_SIZE + 3.0,
        icon_color,
    );
    let text_x = theme::ROW_PAD + ICON_GUTTER;
    painter.text(
        rect.left_top() + egui::vec2(text_x, 7.0),
        egui::Align2::LEFT_TOP,
        &peer.alias,
        egui::FontId::proportional(theme::ROW_FONT),
        theme::INK,
    );
    painter.text(
        rect.left_bottom() + egui::vec2(text_x, -7.0),
        egui::Align2::LEFT_BOTTOM,
        &peer.detail,
        egui::FontId::proportional(theme::DETAIL_FONT),
        theme::DIM,
    );
    if peer.insecure {
        let text = painter.text(
            rect.right_center() - egui::vec2(theme::ROW_PAD, 0.0),
            egui::Align2::RIGHT_CENTER,
            "HTTP",
            egui::FontId::proportional(theme::DETAIL_FONT),
            theme::DANGER,
        );
        icons::paint(
            painter,
            Icon::LockOpen,
            text.left_center() - egui::vec2(theme::ICON_SIZE / 2.0 + 4.0, 0.0),
            theme::ICON_SIZE - 2.0,
            theme::DANGER,
        );
    }
    response
}

/// `[Btn] Action` hints spread evenly across the width — each hint owns an
/// equal slot and sits centered in it, matching the tab bar. Shared by every
/// screen's footer. The button is drawn as the key it names.
pub fn hint_bar(ui: &mut egui::Ui, hints: &[Hint], taps: &mut Vec<AppCommand>) {
    if hints.is_empty() {
        return;
    }
    // Painted over one reserved row, no per-slot widgets: ids shifting
    // between egui's passes paint a red line at the panel edge.
    let items: Vec<(Cap, std::sync::Arc<egui::Galley>)> = hints
        .iter()
        .map(|(button, action, _)| (Cap::for_button(ui, button), action_galley(ui, action)))
        .collect();
    let row_h = items
        .iter()
        .map(|(_, g)| g.size().y)
        .fold(CAP_SIZE, f32::max);
    let full_w = ui.available_width();
    let (_, rect) = ui.allocate_space(egui::vec2(full_w, row_h));
    let slot_w = full_w / hints.len() as f32;
    let tap = tap_pos(ui);
    for (i, (cap, galley)) in items.into_iter().enumerate() {
        let center = egui::pos2(rect.left() + slot_w * (i as f32 + 0.5), rect.center().y);
        // A hint names a button, so its slot *is* that button — the only way to
        // reach Start/Select/X/Y on a device with no pad.
        if let Some(command) = hints[i].2 {
            let slot = egui::Rect::from_center_size(center, egui::vec2(slot_w, row_h))
                .expand2(egui::vec2(0.0, HINT_TAP_PAD));
            if tap.is_some_and(|pos| slot.contains(pos)) {
                taps.push(command);
            }
        }
        let width = cap.width() + HINT_GAP + galley.size().x;
        let left = center.x - width / 2.0;
        cap.paint(ui.painter(), egui::pos2(left + cap.width() / 2.0, center.y));
        let text_pos = egui::pos2(
            left + cap.width() + HINT_GAP,
            center.y - galley.size().y / 2.0,
        );
        ui.painter().galley(text_pos, galley, theme::DIM);
    }
}

/// Where a tap landed, for bars and virtualized lists that paint themselves.
/// `interact_pos` survives the `PointerGone` a lifted finger sends. Filtered
/// to the clip rect and to this layer, as a sensed widget would be.
pub fn tap_pos(ui: &egui::Ui) -> Option<egui::Pos2> {
    ui.input(|i| {
        i.pointer
            .primary_clicked()
            .then(|| i.pointer.interact_pos())
            .flatten()
    })
    .filter(|pos| ui.clip_rect().contains(*pos))
    .filter(|pos| ui.ctx().layer_id_at(*pos) == Some(ui.layer_id()))
}

/// Height of a key cap, and the diameter of a round one.
const CAP_SIZE: f32 = 18.0;
/// Side padding of a wide cap around its label.
const CAP_PAD: f32 = 7.0;
/// The label's type size: small caps on a small key.
const CAP_FONT: f32 = 11.0;
/// Space between a hint's key and its action.
const HINT_GAP: f32 = 6.0;
/// The cap's fill and ring, both the accent faded.
const CAP_FILL: f32 = 0.18;
const CAP_RING: f32 = 0.6;
/// Side of a direction triangle on a cap.
const TRI_SIZE: f32 = 6.0;

/// A direction a cap can show instead of a label.
#[derive(Clone, Copy)]
enum Dir {
    Left,
    Right,
}

/// The key a hint names, drawn the way the pad prints it.
enum Cap {
    /// A face button lettered A/B/X/Y: a round cap with the letter.
    Letter(std::sync::Arc<egui::Galley>),
    /// A face button the pad marks with a shape (the Vita).
    Face(Face),
    /// Select, Start, the shoulders: a wide cap with the name.
    Wide(std::sync::Arc<egui::Galley>),
    /// Direction keys: a cap per triangle.
    Arrows(Vec<Dir>),
}

impl Cap {
    fn for_button(ui: &egui::Ui, button: &str) -> Self {
        if let Some(key) = crate::event::hints::label(button) {
            return if key.chars().count() == 1 {
                Cap::Letter(cap_galley(ui, &key))
            } else {
                Cap::Wide(cap_galley(ui, &key))
            };
        }
        if let Some(face) = pad_face(button) {
            return Cap::Face(face);
        }
        match button {
            "← →" => Cap::Arrows(vec![Dir::Left, Dir::Right]),
            "◂" => Cap::Arrows(vec![Dir::Left]),
            "▸" => Cap::Arrows(vec![Dir::Right]),
            name if name.chars().count() == 1 => Cap::Letter(cap_galley(ui, name)),
            name => Cap::Wide(cap_galley(ui, pad_label(name))),
        }
    }

    fn width(&self) -> f32 {
        match self {
            Cap::Letter(_) | Cap::Face(_) => CAP_SIZE,
            Cap::Wide(galley) => galley.size().x + CAP_PAD * 2.0,
            Cap::Arrows(dirs) => dirs.len() as f32 * CAP_SIZE + (dirs.len() as f32 - 1.0) * 3.0,
        }
    }

    fn paint(&self, painter: &egui::Painter, center: egui::Pos2) {
        let plate = |rect: egui::Rect| {
            painter.rect(
                rect,
                CAP_SIZE / 2.0,
                theme::ACCENT.linear_multiply(CAP_FILL),
                egui::Stroke::new(1.0, theme::ACCENT.linear_multiply(CAP_RING)),
                egui::StrokeKind::Inside,
            );
        };
        match self {
            Cap::Letter(galley) => {
                plate(egui::Rect::from_center_size(
                    center,
                    egui::Vec2::splat(CAP_SIZE),
                ));
                let pos = center - galley.size() / 2.0;
                painter.galley(pos, galley.clone(), theme::ACCENT);
            }
            Cap::Face(face) => {
                plate(egui::Rect::from_center_size(
                    center,
                    egui::Vec2::splat(CAP_SIZE),
                ));
                paint_face(painter, *face, center);
            }
            Cap::Wide(galley) => {
                let size = egui::vec2(galley.size().x + CAP_PAD * 2.0, CAP_SIZE);
                plate(egui::Rect::from_center_size(center, size));
                let pos = center - galley.size() / 2.0;
                painter.galley(pos, galley.clone(), theme::ACCENT);
            }
            Cap::Arrows(dirs) => {
                let left = center.x - self.width() / 2.0;
                for (i, dir) in dirs.iter().enumerate() {
                    let c = egui::pos2(
                        left + CAP_SIZE / 2.0 + i as f32 * (CAP_SIZE + 3.0),
                        center.y,
                    );
                    plate(egui::Rect::from_center_size(c, egui::Vec2::splat(CAP_SIZE)));
                    let h = TRI_SIZE / 2.0;
                    let w = TRI_SIZE * 0.45;
                    let pts = match dir {
                        Dir::Left => vec![
                            c + egui::vec2(w, -h),
                            c + egui::vec2(-w, 0.0),
                            c + egui::vec2(w, h),
                        ],
                        Dir::Right => vec![
                            c + egui::vec2(-w, -h),
                            c + egui::vec2(w, 0.0),
                            c + egui::vec2(-w, h),
                        ],
                    };
                    painter.add(egui::Shape::convex_polygon(
                        pts,
                        theme::ACCENT,
                        egui::Stroke::NONE,
                    ));
                }
            }
        }
    }
}

/// A face button its pad marks with a shape rather than a letter.
#[derive(Clone, Copy)]
#[cfg_attr(not(target_os = "vita"), allow(dead_code))]
enum Face {
    Cross,
    Circle,
    Square,
    Triangle,
}

/// The shape the device's pad prints for a button the hints name by SDL's
/// positional layout (A south, B east, X west, Y north), or by the swapped
/// one when the setting trades the pairs.
#[cfg(target_os = "vita")]
fn pad_face(button: &str) -> Option<Face> {
    let swapped = crate::config::input::swap_abxy();
    match (button, swapped) {
        ("A", false) | ("B", true) => Some(Face::Cross),
        ("B", false) | ("A", true) => Some(Face::Circle),
        ("X", false) | ("Y", true) => Some(Face::Square),
        ("Y", false) | ("X", true) => Some(Face::Triangle),
        _ => None,
    }
}

#[cfg(not(target_os = "vita"))]
fn pad_face(_button: &str) -> Option<Face> {
    None
}

/// The name the device's pad prints for a button without a [`Face`].
fn pad_label(button: &str) -> &str {
    match button {
        "L1 R1" if cfg!(target_os = "vita") => "L R",
        other => other,
    }
}

/// Side of the box a [`Face`] is drawn in, inside its cap.
const FACE_SIZE: f32 = CAP_SIZE * 0.5;
const FACE_STROKE: f32 = 1.5;
/// Share of the box the square spans: drawn to its edges, it reads larger than
/// the circle.
const FACE_ANGULAR_SPAN: f32 = 0.85;
/// Share of the box the cross spans: thin diagonals read smaller than their box.
const CROSS_SPAN: f32 = 1.0;
/// The square's height share: a true square reads squat beside the others.
const SQUARE_HEIGHT_SPAN: f32 = 0.95;

/// Draw `face` centred on `center`, every shape filling the same box.
fn paint_face(painter: &egui::Painter, face: Face, center: egui::Pos2) {
    let half = FACE_SIZE / 2.0;
    let stroke = egui::Stroke::new(FACE_STROKE, theme::ACCENT);
    match face {
        Face::Cross => {
            let d = half * CROSS_SPAN;
            painter.line_segment(
                [center + egui::vec2(-d, -d), center + egui::vec2(d, d)],
                stroke,
            );
            painter.line_segment(
                [center + egui::vec2(-d, d), center + egui::vec2(d, -d)],
                stroke,
            );
        }
        Face::Circle => {
            painter.circle_stroke(center, half, stroke);
        }
        Face::Square => {
            let size = egui::vec2(
                FACE_SIZE * FACE_ANGULAR_SPAN,
                FACE_SIZE * SQUARE_HEIGHT_SPAN,
            );
            let rect = egui::Rect::from_center_size(center, size);
            painter.rect_stroke(rect, 0.0, stroke, egui::StrokeKind::Middle);
        }
        Face::Triangle => {
            // Centred on its bounding box, not its centroid, to sit on the line.
            let h = FACE_SIZE;
            let w = h / 3.0_f32.sqrt();
            let points = vec![
                center + egui::vec2(0.0, -h / 2.0),
                center + egui::vec2(w, h / 2.0),
                center + egui::vec2(-w, h / 2.0),
            ];
            painter.add(egui::Shape::closed_line(points, stroke));
        }
    }
}

fn cap_galley(ui: &egui::Ui, label: &str) -> std::sync::Arc<egui::Galley> {
    ui.painter().layout_no_wrap(
        label.to_string(),
        egui::FontId::proportional(CAP_FONT),
        theme::ACCENT,
    )
}

fn action_galley(ui: &egui::Ui, action: &str) -> std::sync::Arc<egui::Galley> {
    ui.painter().layout_no_wrap(
        action.to_string(),
        egui::FontId::proportional(theme::DETAIL_FONT),
        theme::DIM,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    const SCREEN: egui::Vec2 = egui::vec2(640.0, 480.0);

    /// The taps one click at `pos` produces. Two frames: egui resolves a click
    /// against the previous frame's rects, so the first lays the screen out.
    fn tap_at(data: &HomeData, pos: egui::Pos2) -> Vec<AppCommand> {
        let ctx = egui::Context::default();
        let mut taps = Vec::new();
        for frame in 0..2 {
            taps.clear();
            let input = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, SCREEN)),
                events: if frame == 0 { Vec::new() } else { click(pos) },
                ..Default::default()
            };
            ctx.begin_pass(input);
            let mut root = egui::Ui::new(
                ctx.clone(),
                egui::Id::new("root_ui"),
                egui::UiBuilder::new().max_rect(ctx.content_rect()),
            );
            render(&mut root, data, &mut taps);
            // Nothing paints the frame here, and an unapplied delta panics on drop.
            ctx.end_pass().textures_delta.clear();
        }
        taps
    }

    fn click(pos: egui::Pos2) -> Vec<egui::Event> {
        let button = |pressed| egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        };
        vec![egui::Event::PointerMoved(pos), button(true), button(false)]
    }

    fn data(peers: usize) -> HomeData {
        HomeData {
            peers: (0..peers)
                .map(|i| PeerRow {
                    alias: format!("peer{i}"),
                    detail: "192.168.1.2".to_string(),
                    icon: Icon::Phone,
                    insecure: false,
                })
                .collect(),
            cursor: Some(0),
            ssid: None,
            spin_frame: None,
        }
    }

    #[test]
    fn tapping_a_radar_row_picks_it_and_sends() {
        let top_row = egui::pos2(100.0, 4.0);
        assert_eq!(
            tap_at(&data(3), top_row),
            vec![AppCommand::PickRow(0), AppCommand::Confirm]
        );
    }

    #[test]
    fn tapping_past_the_last_row_does_nothing() {
        let below = egui::pos2(100.0, SCREEN.y / 2.0);
        assert!(tap_at(&data(1), below).is_empty());
    }

    #[test]
    fn tapping_a_hint_presses_the_button_it_names() {
        // Four slots; "Select · Refresh" is the second, "A · Choose files" the last.
        let bar_y = SCREEN.y - 12.0;
        assert_eq!(
            tap_at(&data(1), egui::pos2(SCREEN.x * 0.375, bar_y)),
            vec![AppCommand::ReAnnounce]
        );
        assert_eq!(
            tap_at(&data(1), egui::pos2(SCREEN.x * 0.875, bar_y)),
            vec![AppCommand::Confirm]
        );
    }

    #[test]
    fn the_tabs_hint_names_no_single_button_so_it_stays_inert() {
        let first_slot = egui::pos2(SCREEN.x * 0.125, SCREEN.y - 12.0);
        assert!(tap_at(&data(1), first_slot).is_empty());
    }
}
