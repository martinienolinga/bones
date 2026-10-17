//! Barre latérale gauche à la Procreate : deux curseurs verticaux séparés par
//! un bouton carré, puis annuler / rétablir. En dessous, une carte de
//! raccourcis d'affichage façon Nomad Sculpt.

use bones_core::{Editor, Mode};
use bones_ui::icons::Icon;
use bones_ui::{theme, widgets};

use super::{COLUMN_TOP, EDGE, UiState, block};

pub fn show(ctx: &egui::Context, editor: &mut Editor, state: &mut UiState) -> Vec<egui::Rect> {
    let main = block(
        ctx,
        "sidebar",
        egui::Align2::LEFT_TOP,
        [EDGE, COLUMN_TOP],
        theme::card_frame(),
        |ui| {
            ui.vertical_centered(|ui| {
                ui.spacing_mut().item_spacing.y = 6.0;
                ui.add_space(4.0);
                context_slider(ui, editor);
                if widgets::tool_button(ui, Icon::Frame, false, "Frame the model").clicked() {
                    editor.frame_all();
                }
                let mut radius = state.settings.pick_radius;
                if widgets::vertical_slider_with_icon(
                    ui,
                    &mut radius,
                    10.0..=48.0,
                    Icon::Marker,
                    "Selection radius",
                )
                .changed()
                {
                    state.settings.pick_radius = radius;
                    state.settings_dirty = true;
                }
                ui.add_space(4.0);
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
    );

    let below = main.bottom() + 10.0;
    let shortcuts = block(
        ctx,
        "sidebar-cards",
        egui::Align2::LEFT_TOP,
        [EDGE, below],
        theme::card_frame(),
        |ui| {
            ui.vertical_centered(|ui| {
                ui.spacing_mut().item_spacing.y = 4.0;
                let view = &mut editor.view;
                toggle(
                    ui,
                    Icon::Weights,
                    &mut view.weights,
                    "Weight map of the selected bone",
                );
                toggle(ui, Icon::Mesh, &mut view.mesh, "Show the mesh");
                toggle(ui, Icon::Bone, &mut view.bones, "Show the bones");
            });
        },
    );
    vec![main, shortcuts]
}

/// Premier curseur : longueur de chaîne IK en pose, nombre de vertèbres en
/// auto-rig, masqué en édition.
fn context_slider(ui: &mut egui::Ui, editor: &mut Editor) {
    match editor.mode {
        Mode::Pose => {
            let mut v = editor.chain_len as f32;
            if widgets::vertical_slider_with_icon(ui, &mut v, 1.0..=6.0, Icon::Chain, "IK chain length")
                .changed()
            {
                editor.chain_len = v.round() as usize;
            }
        }
        Mode::AutoRig => {
            let mut v = editor.autorig_options.spine_count as f32;
            if widgets::vertical_slider_with_icon(ui, &mut v, 1.0..=6.0, Icon::Hierarchy, "Spine bones")
                .changed()
            {
                editor.autorig_options.spine_count = v.round() as u8;
            }
        }
        Mode::Edit => {}
    }
}

fn toggle(ui: &mut egui::Ui, icon: Icon, value: &mut bool, hint: &str) {
    if widgets::tool_button(ui, icon, *value, hint).clicked() {
        *value = !*value;
    }
}
