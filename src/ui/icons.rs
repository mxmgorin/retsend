//! Icons painted from primitives: the shapes the chrome needs, drawn in a
//! unit box at one stroke weight, so they stay sharp at any zoom and need no
//! icon font.

use super::theme;
use egui_sdl2::egui;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Icon {
    /// A tray with an arrow dropping into it.
    Receive,
    /// A tray with an arrow leaving it.
    Send,
    /// A clock face.
    History,
    /// A gear.
    Settings,
    Folder,
    File,
    /// Filled: a pinned row.
    Star,
    CheckboxOff,
    CheckboxOn,
    Check,
    Cross,
    Plus,
    Phone,
    Monitor,
    Globe,
    Terminal,
    Server,
    /// A closed padlock: HTTPS.
    Lock,
    /// An open padlock: a peer announcing plain HTTP.
    LockOpen,
    /// Three arcs over a dot.
    Wifi,
    Info,
    Warning,
    /// An open angle pointing left.
    ChevronLeft,
    /// An open angle pointing right.
    ChevronRight,
}

/// The device icon for a LocalSend `deviceType`; anything unknown is a
/// desktop, the type most senders are.
pub fn device_icon(device_type: Option<&str>) -> Icon {
    match device_type {
        Some("mobile") => Icon::Phone,
        Some("web") => Icon::Globe,
        Some("headless") => Icon::Terminal,
        Some("server") => Icon::Server,
        _ => Icon::Monitor,
    }
}

/// Stroke width as a share of the box, with a floor so a small icon stays a line.
const STROKE_SHARE: f32 = 0.11;
const MIN_STROKE: f32 = 1.4;

/// Paint `icon` centered at `center` in a box of `size`, in `color`.
pub fn paint(
    painter: &egui::Painter,
    icon: Icon,
    center: egui::Pos2,
    size: f32,
    color: egui::Color32,
) {
    let p = |x: f32, y: f32| center + egui::vec2(x, y) * size;
    let stroke = egui::Stroke::new((size * STROKE_SHARE).max(MIN_STROKE), color);
    let line = |a: egui::Pos2, b: egui::Pos2| painter.line_segment([a, b], stroke);
    let path = |pts: Vec<egui::Pos2>| {
        painter.add(egui::Shape::line(pts, stroke));
    };
    let closed = |pts: Vec<egui::Pos2>| {
        painter.add(egui::Shape::closed_line(pts, stroke));
    };
    match icon {
        Icon::Receive | Icon::Send => {
            path(vec![
                p(-0.45, 0.12),
                p(-0.45, 0.45),
                p(0.45, 0.45),
                p(0.45, 0.12),
            ]);
            let (tail, head, dir) = if icon == Icon::Receive {
                (p(0.0, -0.48), p(0.0, 0.2), 1.0)
            } else {
                (p(0.0, 0.2), p(0.0, -0.48), -1.0)
            };
            line(tail, head);
            path(vec![
                head + egui::vec2(-0.24, -0.24 * dir) * size,
                head,
                head + egui::vec2(0.24, -0.24 * dir) * size,
            ]);
        }
        Icon::History => {
            painter.circle_stroke(center, size * 0.44, stroke);
            path(vec![p(-0.2, 0.08), p(0.0, 0.0), p(0.0, -0.3)]);
        }
        Icon::Settings => {
            const TEETH: usize = 8;
            let pts = (0..TEETH * 4)
                .map(|i| {
                    let r = if (i / 2) % 2 == 0 { 0.48 } else { 0.35 };
                    let a = (i as f32 + 0.5) / (TEETH * 4) as f32 * std::f32::consts::TAU;
                    p(a.cos() * r, a.sin() * r)
                })
                .collect();
            closed(pts);
            painter.circle_stroke(center, size * 0.14, stroke);
        }
        Icon::Folder => closed(vec![
            p(-0.48, -0.32),
            p(-0.12, -0.32),
            p(-0.02, -0.2),
            p(0.48, -0.2),
            p(0.48, 0.38),
            p(-0.48, 0.38),
        ]),
        Icon::File => {
            closed(vec![
                p(-0.34, -0.46),
                p(0.1, -0.46),
                p(0.34, -0.22),
                p(0.34, 0.46),
                p(-0.34, 0.46),
            ]);
            path(vec![p(0.1, -0.46), p(0.1, -0.22), p(0.34, -0.22)]);
        }
        Icon::Star => {
            const POINTS: usize = 5;
            let mut mesh = egui::Mesh::default();
            mesh.colored_vertex(center, color);
            for i in 0..POINTS * 2 {
                let r = if i % 2 == 0 { 0.5 } else { 0.21 };
                let a = i as f32 / (POINTS * 2) as f32 * std::f32::consts::TAU
                    - std::f32::consts::FRAC_PI_2;
                mesh.colored_vertex(p(a.cos() * r, a.sin() * r), color);
            }
            for i in 0..(POINTS * 2) as u32 {
                mesh.add_triangle(0, 1 + i, 1 + (i + 1) % (POINTS * 2) as u32);
            }
            painter.add(egui::Shape::mesh(mesh));
        }
        Icon::CheckboxOff => {
            let r = egui::Rect::from_center_size(center, egui::Vec2::splat(size * 0.84));
            painter.rect_stroke(r, size * 0.16, stroke, egui::StrokeKind::Inside);
        }
        Icon::CheckboxOn => {
            let r = egui::Rect::from_center_size(center, egui::Vec2::splat(size * 0.84));
            painter.rect_filled(r, size * 0.16, color);
            let tick = egui::Stroke::new(stroke.width, theme::bg());
            painter.add(egui::Shape::line(
                vec![p(-0.24, 0.02), p(-0.08, 0.18), p(0.24, -0.18)],
                tick,
            ));
        }
        Icon::Check => path(vec![p(-0.42, 0.04), p(-0.14, 0.32), p(0.44, -0.3)]),
        Icon::Cross => {
            line(p(-0.34, -0.34), p(0.34, 0.34));
            line(p(-0.34, 0.34), p(0.34, -0.34));
        }
        Icon::Plus => {
            line(p(-0.4, 0.0), p(0.4, 0.0));
            line(p(0.0, -0.4), p(0.0, 0.4));
        }
        Icon::Phone => {
            let r = egui::Rect::from_center_size(center, egui::vec2(size * 0.56, size * 0.94));
            painter.rect_stroke(r, size * 0.1, stroke, egui::StrokeKind::Inside);
            line(p(-0.1, 0.34), p(0.1, 0.34));
        }
        Icon::Monitor => {
            let r = egui::Rect::from_min_max(p(-0.48, -0.42), p(0.48, 0.2));
            painter.rect_stroke(r, size * 0.08, stroke, egui::StrokeKind::Inside);
            line(p(0.0, 0.2), p(0.0, 0.42));
            line(p(-0.26, 0.44), p(0.26, 0.44));
        }
        Icon::Globe => {
            painter.circle_stroke(center, size * 0.46, stroke);
            line(p(-0.46, 0.0), p(0.46, 0.0));
            const SEGMENTS: usize = 24;
            let meridian: Vec<_> = (0..=SEGMENTS)
                .map(|i| {
                    let a = i as f32 / SEGMENTS as f32 * std::f32::consts::TAU;
                    p(a.sin() * 0.2, -a.cos() * 0.46)
                })
                .collect();
            path(meridian);
        }
        Icon::Terminal => {
            let r = egui::Rect::from_min_max(p(-0.48, -0.38), p(0.48, 0.38));
            painter.rect_stroke(r, size * 0.08, stroke, egui::StrokeKind::Inside);
            path(vec![p(-0.3, -0.14), p(-0.12, 0.0), p(-0.3, 0.14)]);
            line(p(0.0, 0.16), p(0.24, 0.16));
        }
        Icon::Server => {
            for (top, bottom) in [(-0.44, -0.06), (0.06, 0.44)] {
                let r = egui::Rect::from_min_max(p(-0.46, top), p(0.46, bottom));
                painter.rect_stroke(r, size * 0.06, stroke, egui::StrokeKind::Inside);
                painter.circle_filled(p(0.3, (top + bottom) / 2.0), stroke.width * 0.8, color);
            }
        }
        Icon::Lock | Icon::LockOpen => {
            let body = egui::Rect::from_min_max(p(-0.34, -0.02), p(0.34, 0.46));
            painter.rect_stroke(body, size * 0.08, stroke, egui::StrokeKind::Inside);
            const SEGMENTS: usize = 12;
            let mut shackle = vec![p(-0.2, -0.02)];
            shackle.extend((0..=SEGMENTS).map(|i| {
                let a = std::f32::consts::PI * (1.0 + i as f32 / SEGMENTS as f32);
                p(a.cos() * 0.2, -0.22 + a.sin() * 0.2)
            }));
            // Open: the right leg stops short of the body.
            let leg_end = if icon == Icon::Lock { -0.02 } else { -0.14 };
            shackle.push(p(0.2, leg_end));
            path(shackle);
        }
        Icon::Wifi => {
            const SEGMENTS: usize = 16;
            for r in [0.2, 0.36, 0.52] {
                let arc = (0..=SEGMENTS)
                    .map(|i| {
                        // A 90° fan opening upward.
                        let a = -std::f32::consts::FRAC_PI_2
                            + std::f32::consts::FRAC_PI_4
                                * (2.0 * i as f32 / SEGMENTS as f32 - 1.0);
                        p(a.cos() * r, 0.36 + a.sin() * r)
                    })
                    .collect();
                path(arc);
            }
            painter.circle_filled(p(0.0, 0.36), stroke.width * 0.8, color);
        }
        Icon::Info => {
            painter.circle_stroke(center, size * 0.46, stroke);
            painter.circle_filled(p(0.0, -0.22), stroke.width * 0.75, color);
            line(p(0.0, -0.04), p(0.0, 0.26));
        }
        Icon::Warning => {
            closed(vec![p(0.0, -0.46), p(0.5, 0.4), p(-0.5, 0.4)]);
            line(p(0.0, -0.14), p(0.0, 0.12));
            painter.circle_filled(p(0.0, 0.27), stroke.width * 0.75, color);
        }
        Icon::ChevronLeft => path(vec![p(0.16, -0.34), p(-0.16, 0.0), p(0.16, 0.34)]),
        Icon::ChevronRight => path(vec![p(-0.16, -0.34), p(0.16, 0.0), p(-0.16, 0.34)]),
    }
}
