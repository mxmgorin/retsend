//! The brand mark painted from primitives: the striped sun of the app icon
//! inside its teal orbit, with the orbit's arrow. Geometry and gradient stops
//! are fractions taken from `resources/icon/retsend-mark.svg`, so the hero on
//! the landing screens is the icon, sharp at any zoom, and its orbit can turn
//! while the device waits.

use super::theme::{self, lerp_color, CORAL, SUN_TOP, TEAL};
use egui_sdl2::egui;
use std::f32::consts::TAU;
use std::time::Duration;

/// Hero height (logical px) on the landing screens.
pub const HERO_SIZE: f32 = 128.0;

/// One turn of the orbit takes this long.
const SPIN_PERIOD: Duration = Duration::from_secs(12);
/// The orbit's frame on a GPU: smooth. A software rasterizer gets a frame
/// every [`SOFTWARE_SPIN_FRAME`] instead, the most it can keep up with.
const GPU_SPIN_FRAME: Duration = Duration::from_millis(33);
const SOFTWARE_SPIN_FRAME: Duration = Duration::from_millis(125);

/// The orbit's frame for the renderer in use, or none when the setting holds
/// it still.
pub fn spin_frame(animate: bool, gpu: bool) -> Option<Duration> {
    if !animate {
        None
    } else if gpu {
        Some(GPU_SPIN_FRAME)
    } else {
        Some(SOFTWARE_SPIN_FRAME)
    }
}

/// Where the orbit is now, in turns, and a request for the frame that keeps it
/// moving; `None` holds it at rest. Read off the clock, not stepped per frame:
/// a frame that comes late then lands where the orbit is, not a step behind.
pub fn turning(ui: &egui::Ui, frame: Option<Duration>) -> f32 {
    let Some(frame) = frame else {
        return 0.0;
    };
    ui.ctx().request_repaint_after(frame);
    (ui.input(|i| i.time) / SPIN_PERIOD.as_secs_f64()).fract() as f32
}

/// The icon's box; every length below is a share of it.
const BOX: f32 = 512.0;
/// The sun's radius.
const SUN_R: f32 = 110.0 / BOX;
/// Where the sun gradient reaches coral, as a share of the diameter.
const WARM_STOP: f32 = 0.3;
/// The slats cut across the sun: `(top, height)` as shares of the diameter.
const SLATS: [(f32, f32); 4] = [
    (0.586, 0.027),
    (0.680, 0.039),
    (0.793, 0.055),
    (0.926, 0.074),
];
/// The coral halo's reach, and its alpha where the sun's edge sits in it.
const HALO_R: f32 = 187.0 / BOX;
const HALO_INNER: f32 = 0.55;
const HALO_ALPHA: f32 = 0.22;

/// The orbit: its radius, the width of its arcs, and the arcs themselves as
/// `(from, to)` in turns, clockwise from three o'clock.
const ORBIT_R: f32 = 192.0 / BOX;
const ARC_W: f32 = 30.0 / BOX;
const ARCS: [(f32, f32); 3] = [
    (-19.0 / 360.0, 19.0 / 360.0),
    (41.0 / 360.0, 79.0 / 360.0),
    (101.0 / 360.0, 139.0 / 360.0),
];
/// The arrow's band: it fades in over the first [`ARROW_FADE`] of its sweep
/// and warms to amber at its head.
const ARROW_FROM: f32 = 192.0 / 360.0;
const ARROW_TO: f32 = 318.0 / 360.0;
const ARROW_W: f32 = 20.0 / BOX;
const ARROW_FADE: f32 = 0.55;
const ARROW_START_ALPHA: f32 = 0.05;
/// The arrowhead: the two arms from its tip, in icon units from the center.
const HEAD_TIP: (f32, f32) = ((398.7 - 256.0) / BOX, (127.5 - 256.0) / BOX);
const HEAD_ARMS: [(f32, f32); 2] = [
    ((389.5 - 256.0) / BOX, (62.2 - 256.0) / BOX),
    ((334.6 - 256.0) / BOX, (111.6 - 256.0) / BOX),
];
/// The amber glow behind the head: its reach and its alpha at the center.
const GLOW_R: f32 = 90.0 / BOX;
const GLOW_INNER: f32 = 0.25;
const GLOW_ALPHA: f32 = 0.3;

/// Segments per turn of an arc polyline, and of a glow ring.
const ARC_SEGMENTS: f32 = 96.0;
const RING_SEGMENTS: usize = 40;
/// Band height of the sun mesh (physical px): fine enough for a round edge.
const BAND_PX: f32 = 1.5;
/// Width of the transparent edge ring that stands in for antialiasing.
const FEATHER_PX: f32 = 1.0;

/// Size the mark occupies at `size`: a square.
pub fn measure(size: f32) -> egui::Vec2 {
    egui::vec2(size, size)
}

/// Paint the mark centered at `center` in a box of `size`. `spin` turns the
/// orbit (in turns, clockwise); `dim` pulls every color toward the plate,
/// 1.0 being gone.
pub fn paint(painter: &egui::Painter, center: egui::Pos2, size: f32, spin: f32, dim: f32) {
    let bg = theme::bg();
    let tint = |c: egui::Color32| lerp_color(c, bg, dim);
    let at = |turns: f32, r: f32| {
        let a = turns * TAU;
        center + egui::vec2(a.cos(), a.sin()) * r * size
    };

    painter.add(egui::Shape::mesh(ring_mesh(
        center,
        HALO_R * HALO_INNER * size,
        HALO_R * size,
        tint(CORAL).gamma_multiply(HALO_ALPHA),
    )));
    painter.add(egui::Shape::mesh(sun_mesh(
        painter.pixels_per_point(),
        center,
        SUN_R * size,
        &tint,
    )));

    let glow_center = center + rotate(egui::vec2(HEAD_TIP.0, HEAD_TIP.1), spin) * size;
    painter.add(egui::Shape::mesh(ring_mesh(
        glow_center,
        GLOW_R * GLOW_INNER * size,
        GLOW_R * size,
        tint(SUN_TOP).gamma_multiply(GLOW_ALPHA),
    )));

    let arc_stroke = egui::Stroke::new(ARC_W * size, tint(TEAL));
    for (from, to) in ARCS {
        let pts = arc_points(|t| at(t + spin, ORBIT_R), from, to);
        round_caps(painter, &pts, arc_stroke);
        painter.add(egui::Shape::line(pts, arc_stroke));
    }

    painter.add(egui::Shape::mesh(arrow_mesh(|t, r| at(t + spin, r), &tint)));
    let head_stroke = egui::Stroke::new(ARC_W * size, tint(SUN_TOP));
    let tip = center + rotate(egui::vec2(HEAD_TIP.0, HEAD_TIP.1), spin) * size;
    for (x, y) in HEAD_ARMS {
        let arm = center + rotate(egui::vec2(x, y), spin) * size;
        round_caps(painter, &[tip, arm], head_stroke);
        painter.line_segment([tip, arm], head_stroke);
    }
}

fn rotate(v: egui::Vec2, turns: f32) -> egui::Vec2 {
    let a = turns * TAU;
    let (s, c) = a.sin_cos();
    egui::vec2(v.x * c - v.y * s, v.x * s + v.y * c)
}

/// Points along an arc from `from` to `to` turns, `at` mapping a turn to a point.
fn arc_points(at: impl Fn(f32) -> egui::Pos2, from: f32, to: f32) -> Vec<egui::Pos2> {
    let steps = ((to - from) * ARC_SEGMENTS).ceil().max(2.0) as usize;
    (0..=steps)
        .map(|i| at(from + (to - from) * i as f32 / steps as f32))
        .collect()
}

/// egui strokes end square; a disc at each end rounds them.
fn round_caps(painter: &egui::Painter, pts: &[egui::Pos2], stroke: egui::Stroke) {
    for p in [pts.first(), pts.last()].into_iter().flatten() {
        painter.circle_filled(*p, stroke.width / 2.0, stroke.color);
    }
}

/// A radial fade: solid `color` out to `inner`, transparent at `outer`.
fn ring_mesh(center: egui::Pos2, inner: f32, outer: f32, color: egui::Color32) -> egui::Mesh {
    let mut mesh = egui::Mesh::default();
    mesh.colored_vertex(center, color);
    for i in 0..RING_SEGMENTS as u32 {
        let a = i as f32 / RING_SEGMENTS as f32 * TAU;
        let dir = egui::vec2(a.cos(), a.sin());
        mesh.colored_vertex(center + dir * inner, color);
        mesh.colored_vertex(center + dir * outer, egui::Color32::TRANSPARENT);
    }
    let n = RING_SEGMENTS as u32;
    for i in 0..n {
        let (a, b) = (1 + 2 * i, 1 + 2 * ((i + 1) % n));
        mesh.add_triangle(0, a, b);
        mesh.add_triangle(a, b, b + 1);
        mesh.add_triangle(a, b + 1, a + 1);
    }
    mesh
}

/// The arrow's band as a quad strip, colored along its sweep.
fn arrow_mesh(
    at: impl Fn(f32, f32) -> egui::Pos2,
    tint: &dyn Fn(egui::Color32) -> egui::Color32,
) -> egui::Mesh {
    let steps = ((ARROW_TO - ARROW_FROM) * ARC_SEGMENTS).ceil() as usize;
    let half = ARROW_W / 2.0;
    let mut mesh = egui::Mesh::default();
    for i in 0..=steps {
        let f = i as f32 / steps as f32;
        let color = if f < ARROW_FADE {
            tint(TEAL)
                .gamma_multiply(ARROW_START_ALPHA + (1.0 - ARROW_START_ALPHA) * f / ARROW_FADE)
        } else {
            tint(lerp_color(
                TEAL,
                SUN_TOP,
                (f - ARROW_FADE) / (1.0 - ARROW_FADE),
            ))
        };
        let t = ARROW_FROM + (ARROW_TO - ARROW_FROM) * f;
        mesh.colored_vertex(at(t, ORBIT_R - half), color);
        mesh.colored_vertex(at(t, ORBIT_R + half), color);
    }
    for i in 0..steps as u32 {
        let (a, b) = (2 * i, 2 * i + 2);
        mesh.add_triangle(a, b, b + 1);
        mesh.add_triangle(a, b + 1, a + 1);
    }
    mesh
}

/// The sun as horizontal bands with the slats left out, colored by the icon
/// gradient, each band's ends feathered.
fn sun_mesh(
    ppp: f32,
    center: egui::Pos2,
    radius: f32,
    tint: &dyn Fn(egui::Color32) -> egui::Color32,
) -> egui::Mesh {
    let d = radius * 2.0;
    let top = center.y - radius;
    let step = (BAND_PX / ppp) / d;
    let feather = FEATHER_PX / ppp;
    let half_width = |f: f32| {
        let dy = (f - 0.5) * d;
        (radius * radius - dy * dy).max(0.0).sqrt()
    };
    let mut mesh = egui::Mesh::default();
    let mut solid = |a: f32, b: f32| {
        let mut f0 = a;
        while f0 < b {
            let f1 = (f0 + step).min(b);
            let rows = [f0, f1].map(|f| (top + f * d, half_width(f), tint(gradient(f))));
            let base = mesh.vertices.len() as u32;
            for (y, w, color) in rows {
                for (dx, c) in [
                    (-w - feather, egui::Color32::TRANSPARENT),
                    (-w, color),
                    (w, color),
                    (w + feather, egui::Color32::TRANSPARENT),
                ] {
                    mesh.colored_vertex(egui::pos2(center.x + dx, y), c);
                }
            }
            for i in 0..3 {
                let (t, b) = (base + i, base + 4 + i);
                mesh.add_triangle(t, t + 1, b + 1);
                mesh.add_triangle(t, b + 1, b);
            }
            f0 = f1;
        }
    };
    let mut from = 0.0;
    for (slat_top, slat_h) in SLATS {
        solid(from, slat_top);
        from = slat_top + slat_h;
    }
    solid(from, 1.0);
    mesh
}

/// The icon gradient at `f` of the way down the disk.
fn gradient(f: f32) -> egui::Color32 {
    if f < WARM_STOP {
        lerp_color(SUN_TOP, CORAL, f / WARM_STOP)
    } else {
        lerp_color(CORAL, TEAL, (f - WARM_STOP) / (1.0 - WARM_STOP))
    }
}

/// The wordmark: `ret` in ink, `send` running teal into coral, as one galley
/// so a centered layout centers it as a unit.
pub fn wordmark(ui: &egui::Ui, size: f32) -> std::sync::Arc<egui::Galley> {
    const INK_PART: &str = "ret";
    const WARM_PART: &str = "send";
    let font = egui::FontId::proportional(size);
    let mut job = egui::text::LayoutJob::default();
    let format = |color| egui::TextFormat {
        font_id: font.clone(),
        color,
        ..Default::default()
    };
    job.append(INK_PART, 0.0, format(theme::ink()));
    let last = WARM_PART.len() - 1;
    for (i, ch) in WARM_PART.chars().enumerate() {
        let color = lerp_color(TEAL, CORAL, i as f32 / last as f32);
        job.append(&ch.to_string(), 0.0, format(color));
    }
    ui.ctx().fonts_mut(|f| f.layout_job(job))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slats_are_ordered_within_the_disk() {
        let mut last_bottom = 0.0;
        for (top, h) in SLATS {
            assert!(top > last_bottom && h > 0.0 && top + h <= 1.0, "{top} {h}");
            last_bottom = top + h;
        }
    }

    #[test]
    fn arcs_and_arrow_stay_apart() {
        let mut spans: Vec<(f32, f32)> = ARCS.to_vec();
        spans.push((ARROW_FROM - 1.0, ARROW_TO - 1.0));
        spans.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
        for pair in spans.windows(2) {
            assert!(pair[0].1 < pair[1].0, "{pair:?}");
        }
    }

    #[test]
    fn sun_mesh_stays_within_the_disk() {
        let center = egui::pos2(100.0, 100.0);
        for ppp in [1.0, 2.0] {
            let feather = FEATHER_PX / ppp;
            let mesh = sun_mesh(ppp, center, 40.0, &|c| c);
            assert!(!mesh.vertices.is_empty());
            for v in &mesh.vertices {
                assert!((v.pos.x - center.x).abs() <= 40.0 + feather + 1e-3);
                assert!((v.pos.y - center.y).abs() <= 40.0 + 1e-3);
            }
        }
    }
}
