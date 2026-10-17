//! Section « Falloff » : formes d'atténuation du pinceau (comme Blender), en
//! vignettes montrant la courbe du centre (gauche, intense) au bord (droite,
//! nul).

use bones_core::Editor;
use bones_core::weight_paint::Falloff;
use bones_ui::{theme, widgets};
use egui::{Color32, Painter, Rect, Shape, Stroke, pos2};

use super::grid;

pub fn show(ui: &mut egui::Ui, editor: &mut Editor) {
    widgets::section_title(ui, "Falloff");
    let current = editor.weight_paint.brush.falloff;
    let mut pick = None;
    grid(ui, Falloff::ALL.len(), |ui, i, cell| {
        let f = Falloff::ALL[i];
        let paint = |p: &Painter, r: Rect| curve(p, r, f);
        if widgets::tile(ui, cell, f.label(), f == current, paint).clicked() {
            pick = Some(f);
        }
    });
    if let Some(f) = pick {
        editor.weight_paint.brush.falloff = f;
    }
    widgets::note(
        ui,
        "Shape of the brush from its centre (left) to its edge (right).",
    );
}

/// Trace la courbe d'atténuation (intensité en fonction de la distance).
fn curve(p: &Painter, r: Rect, falloff: Falloff) {
    const N: usize = 24;
    p.line_segment(
        [r.left_bottom(), r.right_bottom()],
        Stroke::new(1.0, theme::HAIRLINE),
    );
    let points: Vec<egui::Pos2> = (0..=N)
        .map(|i| {
            let t = i as f32 / N as f32;
            let s = falloff.strength(t).clamp(0.0, 1.0);
            pos2(r.left() + t * r.width(), r.bottom() - s * r.height())
        })
        .collect();
    p.add(Shape::line(points, Stroke::new(2.2, Color32::WHITE)));
}
