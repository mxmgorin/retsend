//! The Send tab: nearby devices in the middle, button hints at the bottom.
//! The tab bar above it is drawn by `super::tabs`; our identity moved to the
//! Receive tab.

use super::{theme, wordmark};
use crate::app::AppCommand;
use egui_sdl2::egui;

/// A footer hint: the button, what it does, and the command a tap on its slot
/// stands for — `None` for the ones naming no single command.
pub type Hint<'a> = (&'a str, &'a str, Option<AppCommand>);

/// A hint slot is one text row tall; taps get a little more to aim at.
const HINT_TAP_PAD: f32 = 6.0;

/// A display-ready radar row. `AppUi::update` builds these from the peer
/// registry, keeping this renderer decoupled from the net layer.
pub struct PeerRow {
    pub alias: String,
    /// Shown under the alias, e.g. "Pixel · 192.168.1.23".
    pub detail: String,
    /// Announced plain HTTP — badged on the right. Encryption is the norm, so
    /// only its absence is worth the pixels.
    pub insecure: bool,
}

/// Everything the Send renderer needs, snapshotted by `AppUi::update` outside
/// the egui closure (shared-state locks must not be held while egui borrows
/// `self`).
pub struct HomeData {
    pub peers: Vec<PeerRow>,
    pub cursor: Option<usize>,
}

pub fn render(root: &mut egui::Ui, data: &HomeData, taps: &mut Vec<AppCommand>) {
    egui::Panel::bottom(super::BOTTOM_PANEL_ID).show(root, |ui| {
        ui.add_space(4.0);
        hint_bar(
            ui,
            &[
                ("← →", "Tabs", None),
                ("Select", "Refresh", Some(AppCommand::ReAnnounce)),
                ("X", "Add IP", Some(AppCommand::Alt)),
                ("A", "Choose files", Some(AppCommand::Confirm)),
            ],
            taps,
        );
        ui.add_space(4.0);
    });

    egui::CentralPanel::default().show(root, |ui| {
        if data.peers.is_empty() {
            // Same branded hero as the Receive tab, with a discovery hint.
            const HERO_H: f32 = 150.0; // wordmark + gap + hint, roughly
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
                ui.add_space(10.0);
                // The escape hatch when this network blocks multicast.
                ui.label(
                    egui::RichText::new("Nothing showing up? X adds a device by IP.")
                        .size(theme::DETAIL_FONT)
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
    if peer.insecure {
        painter.text(
            rect.right_center() - egui::vec2(padding, 0.0),
            egui::Align2::RIGHT_CENTER,
            "HTTP",
            egui::FontId::proportional(theme::DETAIL_FONT),
            theme::DANGER,
        );
    }
    response
}

/// `[Btn] Action` hints spread evenly across the width — each hint owns an
/// equal slot and sits centered in it, matching the tab bar. Shared by every
/// screen's footer.
pub fn hint_bar(ui: &mut egui::Ui, hints: &[Hint], taps: &mut Vec<AppCommand>) {
    if hints.is_empty() {
        return;
    }
    // Painted directly over one reserved row — no nested layout, no per-slot
    // interactive widgets. The hint count differs per tab, and any widget id
    // shifting between egui's passes paints a red line at the panel edge. Taps
    // are hit-tested against the slots for the same reason.
    let galleys: Vec<_> = hints
        .iter()
        .map(|(button, action, _)| hint_galley(ui, button, action))
        .collect();
    let row_h = galleys.iter().map(|g| g.size().y).fold(0.0_f32, f32::max);
    let full_w = ui.available_width();
    let (_, rect) = ui.allocate_space(egui::vec2(full_w, row_h));
    let slot_w = full_w / hints.len() as f32;
    let tap = tap_pos(ui);
    for (i, galley) in galleys.into_iter().enumerate() {
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
        ui.painter()
            .galley(center - galley.size() / 2.0, galley, theme::DIM);
    }
}

/// Where a tap landed, for the bars and virtualized lists that paint themselves
/// instead of allocating widgets. `interact_pos` survives the `PointerGone` a
/// lifted finger sends, so it reads touches too.
///
/// The filters are what a sensed widget gets from egui for free: a rect can run
/// past its clip rect (virtualized rows, padded hint slots), and a layer above
/// owns the tap (the incoming-request modal).
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
                    insecure: false,
                })
                .collect(),
            cursor: Some(0),
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
