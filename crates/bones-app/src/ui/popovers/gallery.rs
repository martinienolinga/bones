//! Galerie : nouveau document, enregistrement, export, et fichiers du
//! dossier Documents (visible dans l'app Fichiers de l'iPad).

use bones_core::Editor;
use bones_ui::theme;

use super::{UiAction, UiState, row_button};

pub fn show(ui: &mut egui::Ui, editor: &mut Editor, state: &mut UiState, actions: &mut Vec<UiAction>) {
    ui.horizontal(|ui| {
        ui.label("Nom");
        ui.text_edit_singleline(&mut editor.rig.name);
    });
    ui.add_space(4.0);
    ui.columns(3, |cols| {
        if cols[0].button("Nouveau").clicked() {
            actions.push(UiAction::NewDemo);
        }
        if cols[1].button("Enregistrer").clicked() {
            actions.push(UiAction::Save);
        }
        if cols[2].button("Exporter .glb").clicked() {
            actions.push(UiAction::ExportGlb);
        }
    });
    ui.add_space(8.0);

    let (rigs, meshes) = state.files().clone();
    egui::ScrollArea::vertical().max_height(420.0).show(ui, |ui| {
        section(ui, "Documents (.bones)", rigs.is_empty());
        for file in rigs {
            if row_button(ui, &file.name).clicked() {
                actions.push(UiAction::Open(file.path));
            }
        }
        ui.add_space(8.0);
        section(ui, "Modèles à importer (.obj)", meshes.is_empty());
        for file in meshes {
            if row_button(ui, &file.name).clicked() {
                actions.push(UiAction::ImportObj(file.path));
            }
        }
    });
    ui.add_space(6.0);
    ui.label(
        egui::RichText::new("Déposez vos fichiers .obj dans « Sur mon iPad › Bones » depuis l'app Fichiers.")
            .small()
            .color(theme::TEXT_DIM),
    );
}

fn section(ui: &mut egui::Ui, title: &str, empty: bool) {
    ui.label(egui::RichText::new(title).strong());
    if empty {
        ui.label(egui::RichText::new("Aucun fichier").color(theme::TEXT_DIM));
    }
}
