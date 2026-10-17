//! Galerie : nouveau document, enregistrement, et fichiers du dossier
//! Documents (visible dans l'app Fichiers de l'iPad) — documents `.bones` à
//! ouvrir et modèles 3D à importer, quel que soit leur format.

use bones_core::Editor;
use bones_ui::theme;

use super::{UiAction, UiState, row_button};
use crate::files::FileEntry;

pub fn show(ui: &mut egui::Ui, editor: &mut Editor, state: &mut UiState, actions: &mut Vec<UiAction>) {
    ui.horizontal(|ui| {
        ui.label("Nom");
        ui.text_edit_singleline(&mut editor.rig.name);
    });
    ui.add_space(4.0);
    ui.columns(2, |cols| {
        if cols[0].button("Nouveau mannequin").clicked() {
            actions.push(UiAction::NewDemo);
        }
        if cols[1].button("Enregistrer").clicked() {
            actions.push(UiAction::Save);
        }
    });
    ui.add_space(8.0);

    let (documents, models) = state.files().clone();
    egui::ScrollArea::vertical().max_height(440.0).show(ui, |ui| {
        section(ui, "Documents", &documents, actions);
        ui.add_space(8.0);
        section(ui, "Modèles et animations à importer", &models, actions);
    });
    ui.add_space(6.0);
    ui.label(
        egui::RichText::new(
            "Déposez vos fichiers (glTF, GLB, FBX, OBJ + MTL, STL, PLY, BVH) et leurs textures dans \
             « Sur mon iPad › Bones » depuis l'app Fichiers. Les exports sont rangés dans « Exports ».",
        )
        .small()
        .color(theme::TEXT_DIM),
    );
}

fn section(ui: &mut egui::Ui, title: &str, files: &[FileEntry], actions: &mut Vec<UiAction>) {
    ui.label(egui::RichText::new(title).strong());
    if files.is_empty() {
        ui.label(egui::RichText::new("Aucun fichier").color(theme::TEXT_DIM));
    }
    for file in files {
        let text = format!("{}   ·   {}", file.name, file.format.extension().to_uppercase());
        if row_button(ui, &text).on_hover_text(file.format.label()).clicked() {
            actions.push(UiAction::Import(file.path.clone()));
        }
    }
}
