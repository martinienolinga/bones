//! Sélection d'une partie du manipulateur sous le doigt.
//!
//! Priorités : flèches, carrés, cubes et centre d'abord, puis anneaux, puis
//! le fond du trackball ; à priorité égale, le plus proche l'emporte, et une
//! moitié avant d'anneau passe devant une moitié arrière.

use glam::Vec2;

use super::Part;
use super::geometry::{Geometry, Kind};

/// Pénalités (en fraction du rayon de sélection) par classe et pour l'arrière.
const CLASS_PENALTY: f32 = 0.35;
const BACK_PENALTY: f32 = 0.25;

fn class(part: Part) -> f32 {
    match part {
        Part::Ring(_) | Part::ViewRing => 1.0,
        Part::Trackball => 2.0,
        _ => 0.0,
    }
}

impl Geometry {
    /// Partie sous `pos` (pixels), à moins de `radius` pixels.
    pub fn pick(&self, pos: Vec2, radius: f32) -> Option<Part> {
        let mut best: Option<(f32, Part)> = None;
        for e in self.elements() {
            let Some(part) = e.part else { continue };
            let points = self.points(e);
            let inside = (e.kind == Kind::Fill || e.solid) && inside_convex(points, pos);
            let d = if inside {
                0.0
            } else if part == Part::Trackball {
                continue; // seul l'intérieur compte
            } else {
                polyline_distance(points, pos, e.kind != Kind::Line)
            };
            if d > radius {
                continue;
            }
            let back = if e.back { BACK_PENALTY * radius } else { 0.0 };
            let score = d + class(part) * CLASS_PENALTY * radius + back;
            // À égalité, l'élément dessiné en dernier (au-dessus) l'emporte.
            if best.is_none_or(|(s, _)| score <= s) {
                best = Some((score, part));
            }
        }
        best.map(|(_, part)| part)
    }
}

/// Distance d'un point à une polyligne (fermée si `closed`).
pub(super) fn polyline_distance(points: &[Vec2], p: Vec2, closed: bool) -> f32 {
    let n = points.len();
    match n {
        0 => f32::INFINITY,
        1 => points[0].distance(p),
        _ => {
            let segments = if closed { n } else { n - 1 };
            (0..segments)
                .map(|i| segment_distance(p, points[i], points[(i + 1) % n]))
                .fold(f32::INFINITY, f32::min)
        }
    }
}

pub(super) fn segment_distance(p: Vec2, a: Vec2, b: Vec2) -> f32 {
    let ab = b - a;
    let len2 = ab.length_squared();
    let t = if len2 > 1e-12 {
        ((p - a).dot(ab) / len2).clamp(0.0, 1.0)
    } else {
        0.0
    };
    p.distance(a + ab * t)
}

/// Point dans un polygone convexe (sens de parcours quelconque).
pub(super) fn inside_convex(points: &[Vec2], p: Vec2) -> bool {
    let n = points.len();
    if n < 3 {
        return false;
    }
    let mut sign = 0.0f32;
    for i in 0..n {
        let (a, b) = (points[i], points[(i + 1) % n]);
        let cross = (b - a).perp_dot(p - a);
        if cross.abs() < 1e-6 {
            continue;
        }
        if sign == 0.0 {
            sign = cross.signum();
        } else if cross.signum() != sign {
            return false;
        }
    }
    sign != 0.0
}
