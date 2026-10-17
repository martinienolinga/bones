//! Hiérarchie des os, propriétés de l'os sélectionné et bascules IK/FK des
//! membres (comme le panneau de contrôle d'Auto-Rig Pro).

use bones_core::{BoneId, Editor, LimbMode, Mode};
use bones_ui::{theme, widgets};

use super::UiState;

pub fn show(ui: &mut egui::Ui, editor: &mut Editor, state: &mut UiState) {
    egui::ScrollArea::vertical()
        .id_salt("bones-tree")
        .max_height(300.0)
        .show(ui, |ui| tree(ui, editor));
    ui.separator();
    properties(ui, editor, state);
    if !editor.rig.limbs.is_empty() {
        ui.separator();
        limbs(ui, editor);
    }
}

fn tree(ui: &mut egui::Ui, editor: &mut Editor) {
    let mut clicked = None;
    let skeleton = &editor.rig.skeleton;
    for id in skeleton.ids() {
        let depth = skeleton.depth(id) as f32;
        ui.horizontal(|ui| {
            ui.add_space(depth * 14.0);
            let selected = editor.selected == Some(id);
            if ui.selectable_label(selected, &skeleton.bone(id).name).clicked() {
                clicked = Some(id);
            }
        });
    }
    if let Some(id) = clicked {
        editor.select(Some(id));
    }
}

fn properties(ui: &mut egui::Ui, editor: &mut Editor, state: &mut UiState) {
    let Some(bone) = editor.selected else {
        ui.label(egui::RichText::new("Touchez un os pour le sélectionner.").color(theme::TEXT_DIM));
        return;
    };
    let b = editor.rig.skeleton.bone(bone);
    let (name, length) = (b.name.clone(), b.length);
    let parent = b.parent().map(|p| editor.rig.skeleton.bone(p).name.clone());
    if state.rename.is_empty() || !ui.memory(|m| m.has_focus(egui::Id::unique("rename"))) {
        state.rename = name;
    }
    ui.horizontal(|ui| {
        ui.label("Nom");
        let edit = egui::TextEdit::singleline(&mut state.rename).id(egui::Id::unique("rename"));
        let response = ui.add(edit);
        if response.lost_focus() {
            let name = state.rename.clone();
            editor.rename_selected(&name);
        }
    });
    ui.label(format!(
        "Parent : {}",
        parent.as_deref().unwrap_or("aucun (racine)")
    ));
    ui.label(format!("Longueur : {length:.3} m"));
    ui.horizontal(|ui| {
        if editor.mode == Mode::Pose && ui.button("Remettre au repos").clicked() {
            editor.reset_pose(true);
        }
        if editor.mode == Mode::Edit && ui.button("Supprimer l'os").clicked() {
            editor.delete_selected();
        }
        if ui.button("Désélectionner").clicked() {
            editor.select(None::<BoneId>);
        }
    });
}

fn limbs(ui: &mut egui::Ui, editor: &mut Editor) {
    ui.label(egui::RichText::new("Membres").strong());
    for i in 0..editor.rig.limbs.len() {
        let limb = &editor.rig.limbs[i];
        let mut index = usize::from(limb.mode == LimbMode::Fk);
        let name = limb.name.clone();
        ui.horizontal(|ui| {
            ui.label(name);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if widgets::segmented(ui, &["IK", "FK"], &mut index).changed() {
                    editor.toggle_limb_mode(i);
                }
            });
        });
    }
}
