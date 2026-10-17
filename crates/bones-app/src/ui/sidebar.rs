//! Bandeau latéral gauche, comme celui de Procreate : collé au bord, centré
//! verticalement ; deux curseurs gris séparés par un petit carré cerclé
//! (cadrer le modèle), puis annuler / rétablir.

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
                context_slider(ui, editor);
                if widgets::square_button(ui, "Frame the model (F)").clicked() {
                    editor.frame_all();
                }
                let mut radius = state.settings.pick_radius;
                if widgets::vertical_slider(ui, &mut radius, 10.0..=48.0, "Selection radius").changed() {
                    state.settings.pick_radius = radius;
                    state.settings_dirty = true;
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

/// Premier curseur : longueur de chaîne IK en pose, nombre de vertèbres en
/// auto-rig ; absent en édition.
fn context_slider(ui: &mut egui::Ui, editor: &mut Editor) {
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
        Mode::Edit => {}
    }
}
