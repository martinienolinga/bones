//! Barre latérale gauche à la Procreate : deux curseurs verticaux séparés par
//! un bouton carré, puis annuler / rétablir. En dessous, des cartes de
//! raccourcis façon Nomad Sculpt.

use bones_core::{Editor, Mode};
use bones_ui::icons::Icon;
use bones_ui::{theme, widgets};

use super::{UiState, block};

const TOP: f32 = 76.0;

pub fn show(ctx: &egui::Context, editor: &mut Editor, state: &mut UiState) -> Vec<egui::Rect> {
    let main = block(
        ctx,
        "sidebar",
        egui::Align2::LEFT_TOP,
        [0.0, TOP],
        theme::card_frame(),
        |ui| {
            ui.vertical_centered(|ui| {
                context_slider(ui, editor);
                ui.add_space(6.0);
                if widgets::tool_button(ui, Icon::Frame, false, "Cadrer le modèle").clicked() {
                    editor.frame_all();
                }
                ui.add_space(6.0);
                let mut radius = state.pick_radius;
                if widgets::vertical_slider(ui, &mut radius, 10.0..=48.0, "Rayon de sélection").changed() {
                    state.pick_radius = radius;
                }
                ui.add_space(10.0);
                let undo = widgets::tool_button(ui, Icon::Undo, false, "Annuler (tape à deux doigts)");
                if undo.clicked() && editor.can_undo() {
                    editor.undo();
                }
                let redo = widgets::tool_button(ui, Icon::Redo, false, "Rétablir (tape à trois doigts)");
                if redo.clicked() && editor.can_redo() {
                    editor.redo();
                }
            });
        },
    );

    let below = main.bottom() + 8.0;
    let shortcuts = block(
        ctx,
        "sidebar-cards",
        egui::Align2::LEFT_TOP,
        [0.0, below],
        theme::card_frame(),
        |ui| {
            ui.vertical_centered(|ui| {
                let view = &mut editor.view;
                toggle(
                    ui,
                    Icon::Weights,
                    &mut view.weights,
                    "Carte de poids de l'os sélectionné",
                );
                toggle(ui, Icon::Mesh, &mut view.mesh, "Afficher le maillage");
                toggle(ui, Icon::Bone, &mut view.bones, "Afficher les os");
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
            if widgets::vertical_slider(ui, &mut v, 1.0..=6.0, "Longueur de chaîne IK").changed() {
                editor.chain_len = v.round() as usize;
            }
        }
        Mode::AutoRig => {
            let mut v = editor.autorig_options.spine_count as f32;
            if widgets::vertical_slider(ui, &mut v, 1.0..=6.0, "Vertèbres").changed() {
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
