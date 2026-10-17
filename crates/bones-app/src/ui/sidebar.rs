//! Bandeau latéral gauche, comme celui de Procreate : collé au bord, centré
//! verticalement ; deux curseurs gris séparés par un petit carré cerclé
//! (cadrer le modèle), puis annuler / rétablir.

use bones_core::weight_paint::PaintTool;
use bones_core::{Editor, Mode};
use bones_ui::icons::Icon;
use bones_ui::{theme, widgets};

use super::{BAR_H, UiState, block};

pub fn show(ctx: &egui::Context, editor: &mut Editor, state: &mut UiState) -> egui::Rect {
    let frame = egui::Frame::new()
        .fill(theme::BAR)
        .corner_radius(egui::CornerRadius {
            nw: 0,
            sw: 0,
            ne: 10,
            se: 10,
        })
        .inner_margin(egui::Margin {
            left: 2,
            right: 2,
            top: 12,
            bottom: 6,
        });
    block(
        ctx,
        "sidebar",
        egui::Align2::LEFT_CENTER,
        [0.0, BAR_H * 0.5],
        frame,
        |ui| {
            ui.vertical_centered(|ui| {
                ui.spacing_mut().item_spacing.y = 6.0;
                if editor.mode == Mode::WeightPaint {
                    brush_sliders(ui, editor);
                    sample_button(ui, editor);
                } else {
                    let first = context_slider(ui, editor);
                    if first {
                        frame_button(ui, editor);
                    }
                    let mut radius = state.settings.pick_radius;
                    if widgets::vertical_slider(ui, &mut radius, 10.0..=48.0, "Selection radius").changed() {
                        state.settings.pick_radius = radius;
                        state.settings_dirty = true;
                    }
                    if !first {
                        frame_button(ui, editor);
                    }
                }
                ui.add_space(2.0);
                ui.spacing_mut().item_spacing.y = 0.0;
                let undo = ui
                    .add_enabled_ui(editor.can_undo(), |ui| {
                        widgets::tool_button(ui, Icon::Undo, false, "Undo (two-finger tap)")
                    })
                    .inner;
                if undo.clicked() {
                    editor.undo();
                }
                let redo = ui
                    .add_enabled_ui(editor.can_redo(), |ui| {
                        widgets::tool_button(ui, Icon::Redo, false, "Redo (three-finger tap)")
                    })
                    .inner;
                if redo.clicked() {
                    editor.redo();
                }
            });
        },
    )
}

/// Petit carré cerclé : cadrer le modèle.
fn frame_button(ui: &mut egui::Ui, editor: &mut Editor) {
    if widgets::square_button(ui, "Frame the model (F)").clicked() {
        editor.frame_all();
    }
}

/// Deux curseurs sobres, comme les autres modes : taille du pinceau et poids
/// cible (valeur numérique 0,00 à 1,00 ; aucune couleur). La force n'est pas
/// ici : elle vient de la pression de l'Apple Pencil (et du réglage de repli
/// des options de trait).
fn brush_sliders(ui: &mut egui::Ui, editor: &mut Editor) {
    let brush = &mut editor.weight_paint.brush;
    let mut radius = brush.radius;
    if widgets::vertical_slider(ui, &mut radius, 4.0..=200.0, "Brush size").changed() {
        brush.radius = radius;
    }
    let mut weight = brush.weight;
    if widgets::vertical_slider_value(ui, &mut weight, 0.0..=1.0, "Max weight", 2).changed() {
        brush.weight = weight;
    }
}

/// Pipette (outil « Sample » de Blender) : petit bouton monochrome, bleu
/// quand il est actif ; un toucher bascule entre pipette et dessin.
fn sample_button(ui: &mut egui::Ui, editor: &mut Editor) {
    let sampling = editor.weight_paint.brush.tool == PaintTool::Sample;
    if widgets::tool_button(ui, Icon::Sample, sampling, "Sample weight (eyedropper)").clicked() {
        editor.weight_paint.brush.tool = if sampling {
            PaintTool::Draw
        } else {
            PaintTool::Sample
        };
    }
}

/// Premier curseur : longueur de chaîne IK en pose, nombre de vertèbres en
/// auto-rig ; absent en édition (retourne alors faux).
fn context_slider(ui: &mut egui::Ui, editor: &mut Editor) -> bool {
    match editor.mode {
        Mode::Pose => {
            let mut v = editor.chain_len as f32;
            if widgets::vertical_slider(ui, &mut v, 1.0..=6.0, "IK chain length").changed() {
                editor.chain_len = v.round() as usize;
            }
        }
        Mode::AutoRig => {
            let mut v = editor.autorig_options.spine_count as f32;
            if widgets::vertical_slider(ui, &mut v, 1.0..=6.0, "Spine bones").changed() {
                editor.autorig_options.spine_count = v.round() as u8;
            }
        }
        // La peinture de poids a ses propres curseurs ([`brush_sliders`]).
        Mode::Edit | Mode::WeightPaint => return false,
    }
    true
}
