//! Bibliothèque de pinceaux de la peinture de poids, en studio à la Brush
//! Studio de Procreate : une colonne de navigation (Brush, Falloff, Stroke)
//! et, à droite, une grille de vignettes à aperçu de chaleur et les réglages
//! du trait. Les pinceaux reprennent ceux du mode Weight Paint de Blender.

mod falloff;
mod library;
mod stroke;

use bones_core::Editor;
use bones_core::weight_paint::PaintTool;
use bones_ui::icons::Icon;
use bones_ui::widgets;
use egui::{Ui, Vec2, vec2};

use crate::ui::UiState;

const SECTIONS: [(Icon, &str); 3] = [
    (Icon::Brush, "Brush"),
    (Icon::Adjust, "Falloff"),
    (Icon::Trajectory, "Stroke"),
];

/// Glyphe de l'outil de peinture (bouton de la barre, vignettes).
pub fn tool_icon(tool: PaintTool) -> Icon {
    match tool {
        PaintTool::Draw => Icon::Brush,
        PaintTool::Blur => Icon::Blur,
        PaintTool::Average => Icon::Average,
        PaintTool::Smear => Icon::Smear,
        PaintTool::Gradient => Icon::Gradient,
        PaintTool::Sample => Icon::Sample,
    }
}

/// Affiche le studio ; retourne vrai pour refermer (bouton « Done »).
pub fn show(ui: &mut egui::Ui, editor: &mut Editor, state: &mut UiState, height: f32) -> bool {
    let mut close = false;
    let mut section = state.brush_section;
    super::studio::show(
        ui,
        "Brushes",
        &SECTIONS,
        &mut section,
        height,
        |ui| close = widgets::confirm_button(ui, "Done").clicked(),
        |ui, section| match section {
            0 => library::show(ui, editor),
            1 => falloff::show(ui, editor),
            _ => stroke::show(ui, editor),
        },
    );
    state.brush_section = section;
    close
}

/// Grille de vignettes (trois par ligne) ; `each` dessine la tuile d'indice
/// `i` à la taille fournie.
pub(super) fn grid(ui: &mut Ui, count: usize, mut each: impl FnMut(&mut Ui, usize, Vec2)) {
    const COLS: usize = 3;
    const GAP: f32 = 10.0;
    let avail = ui.available_width();
    let w = ((avail - GAP * (COLS as f32 - 1.0)) / COLS as f32).max(80.0);
    let cell = vec2(w, w * 0.82 + 18.0);
    for row in 0..count.div_ceil(COLS) {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = GAP;
            for col in 0..COLS {
                let i = row * COLS + col;
                if i < count {
                    each(ui, i, cell);
                }
            }
        });
        ui.add_space(GAP);
    }
}
