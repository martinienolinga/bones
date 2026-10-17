//! Studio des os : arbre du squelette, propriétés de l'os sélectionné
//! (nom, déformation, liaison, symétrie), position, parent, outils (copie,
//! subdivision, suppression) et bascules IK/FK des membres.
//!
//! Les retouches du squelette sont des actions du mode Édition : en Pose,
//! les valeurs restent visibles mais en lecture seule, avec un bouton pour
//! passer en Édition.

mod bone;
mod hierarchy;
mod tools;
mod transform;

use bones_core::editor::BoneInfo;
use bones_core::{Editor, Mode};
use bones_ui::icons::Icon;
use bones_ui::{theme, widgets};

use super::{note, studio};
use crate::ui::UiState;

const SECTIONS: [(Icon, &str); 6] = [
    (Icon::Hierarchy, "Skeleton"),
    (Icon::Bone, "Bone"),
    (Icon::Move, "Transform"),
    (Icon::Chain, "Parent"),
    (Icon::Duplicate, "Tools"),
    (Icon::Ik, "Limbs"),
];

/// Retourne vrai quand le studio doit se fermer.
pub fn show(ui: &mut egui::Ui, editor: &mut Editor, state: &mut UiState, height: f32) -> bool {
    let mut close = false;
    let mut section = state.bones_section;
    studio::show(
        ui,
        "Bones",
        &SECTIONS,
        &mut section,
        height,
        |ui| close = widgets::confirm_button(ui, "Done").clicked(),
        |ui, section| match section {
            0 => hierarchy::tree(ui, editor),
            5 => hierarchy::limbs(ui, editor),
            _ => selected(ui, editor, state, section),
        },
    );
    state.bones_section = section;
    close
}

/// Sections propres à l'os sélectionné.
fn selected(ui: &mut egui::Ui, editor: &mut Editor, state: &mut UiState, section: usize) {
    let Some(id) = editor.selected else {
        note(ui, "Tap a bone in the view, or in Skeleton, to select it.");
        return;
    };
    let info = editor.bone_info(id);
    navigation(ui, editor, &info);
    let editing = editor.mode == Mode::Edit;
    if !editing {
        pose_banner(ui, editor);
    }
    ui.add_enabled_ui(editing, |ui| {
        if !editing {
            ui.set_opacity(0.55);
        }
        match section {
            1 => bone::show(ui, editor, state, &info),
            2 => transform::show(ui, editor, &info),
            3 => hierarchy::parent(ui, editor, &info),
            _ => tools::show(ui, editor, state),
        }
    });
}

/// Nom de l'os et déplacements dans la hiérarchie (parent, enfant, frère).
fn navigation(ui: &mut egui::Ui, editor: &mut Editor, info: &BoneInfo) {
    ui.horizontal(|ui| {
        ui.add_space(16.0);
        ui.label(
            egui::RichText::new(&info.name)
                .font(theme::semibold(17.0))
                .color(theme::BONE_SELECTED),
        );
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.spacing_mut().item_spacing.x = 6.0;
            if widgets::chip_button(ui, "Next →", false)
                .on_hover_text("Next sibling (N)")
                .clicked()
            {
                editor.select_next_sibling();
            }
            let child = ui.add_enabled_ui(info.children > 0, |ui| widgets::chip_button(ui, "Child ↓", false));
            if child.inner.on_hover_text("First child (C)").clicked() {
                editor.select_child();
            }
            let parent = ui.add_enabled_ui(info.parent.is_some(), |ui| {
                widgets::chip_button(ui, "Parent ↑", false)
            });
            if parent.inner.on_hover_text("Parent (P)").clicked() {
                editor.select_parent();
            }
        });
    });
}

/// En mode Pose : rappel que le squelette s'édite en mode Édition.
fn pose_banner(ui: &mut egui::Ui, editor: &mut Editor) {
    widgets::group(ui, |g| {
        g.custom(52.0, |ui| {
            ui.label(
                egui::RichText::new("Skeleton changes happen in Edit mode.")
                    .size(15.0)
                    .color(theme::TEXT_LIST),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if widgets::chip_button(ui, "Edit skeleton", true).clicked() {
                    editor.set_mode(Mode::Edit);
                }
            });
        });
    });
}
