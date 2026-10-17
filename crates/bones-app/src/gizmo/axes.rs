//! Guide à axes, style Blender : pastilles pleines et étiquetées pour les
//! axes positifs, cercles creux pour les axes négatifs, dessinés du plus
//! lointain au plus proche.

use glam::Vec3;

use super::{AXIS_COLORS, Basis};

const LABELS: [&str; 3] = ["X", "Y", "Z"];
const BALL: f32 = 11.0;
const HOLLOW: f32 = 8.5;

/// Dessine le guide ; retourne l'axe sous `hit` (pastille la plus proche
/// de l'observateur), s'il y en a un.
pub fn paint(
    painter: &egui::Painter,
    center: egui::Pos2,
    radius: f32,
    basis: &Basis,
    hit: Option<egui::Pos2>,
) -> Option<Vec3> {
    let axes = [Vec3::X, Vec3::Y, Vec3::Z];
    let mut items: Vec<(f32, usize, bool, egui::Pos2)> = Vec::with_capacity(6);
    for (i, axis) in axes.iter().enumerate() {
        for positive in [true, false] {
            let dir = if positive { *axis } else { -*axis };
            let (offset, depth) = basis.project(dir);
            items.push((depth, i, positive, center + offset * radius));
        }
    }
    items.sort_by(|a, b| a.0.total_cmp(&b.0));

    let mut picked = None;
    for &(depth, i, positive, pos) in &items {
        // Les éléments lointains sont légèrement estompés.
        let fade = 0.55 + 0.45 * (depth * 0.5 + 0.5);
        let color = AXIS_COLORS[i].gamma_multiply(fade);
        if positive {
            painter.line_segment([center, pos], egui::Stroke::new(2.6, color));
            painter.circle_filled(pos, BALL, color);
            painter.circle_stroke(
                pos,
                BALL,
                egui::Stroke::new(1.0, egui::Color32::from_white_alpha(40)),
            );
            painter.text(
                pos,
                egui::Align2::CENTER_CENTER,
                LABELS[i],
                egui::FontId::proportional(13.0),
                egui::Color32::from_black_alpha(220),
            );
        } else {
            painter.circle_filled(pos, HOLLOW, AXIS_COLORS[i].gamma_multiply(0.22 * fade));
            painter.circle_stroke(pos, HOLLOW, egui::Stroke::new(1.6, color));
        }
        let size = if positive { BALL } else { HOLLOW } + 3.0;
        if hit.is_some_and(|h| h.distance(pos) <= size) {
            let dir = axes[i] * if positive { 1.0 } else { -1.0 };
            picked = Some(dir);
        }
    }
    // Point central.
    painter.circle_filled(center, 2.5, egui::Color32::from_white_alpha(160));
    picked
}
