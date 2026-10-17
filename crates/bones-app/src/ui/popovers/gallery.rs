//! Galerie : nouveau document, enregistrement, et fichiers du dossier
//! Documents (visible dans l'app Fichiers de l'iPad) — documents `.bones` à
//! ouvrir et modèles 3D à importer, quel que soit leur format.

use bones_core::Editor;
use bones_core::io::Format;
use bones_ui::icons::Icon;
use bones_ui::{theme, widgets};

use super::{UiAction, UiState};
use crate::files::FileEntry;

pub fn show(ui: &mut egui::Ui, editor: &mut Editor, state: &mut UiState, actions: &mut Vec<UiAction>) {
    ui.add(
        egui::TextEdit::singleline(&mut editor.rig.name)
            .hint_text("Nom du document")
            .desired_width(f32::INFINITY)
            .margin(egui::Margin::symmetric(12, 10)),
    );
    ui.add_space(4.0);
    ui.horizontal(|ui| {
        if widgets::pill_button(ui, Some(Icon::Pose), "Nouveau mannequin", false).clicked() {
            actions.push(UiAction::NewDemo);
        }
        if widgets::pill_button(ui, Some(Icon::Save), "Enregistrer", true).clicked() {
            actions.push(UiAction::Save);
        }
    });
    ui.add_space(6.0);

    let (documents, models) = state.files().clone();
    egui::ScrollArea::vertical().max_height(430.0).show(ui, |ui| {
        ui.spacing_mut().item_spacing.y = 4.0;
        section(ui, "Documents", &documents, actions);
        ui.add_space(6.0);
        section(ui, "Modèles et animations à importer", &models, actions);
    });
    ui.add_space(6.0);
    ui.label(
        egui::RichText::new(
            "Déposez vos fichiers (glTF, GLB, FBX, OBJ + MTL, STL, PLY, BVH) et leurs textures dans \
             « Sur mon iPad › Bones » depuis l'app Fichiers. Les exports sont rangés dans « Exports ».",
        )
        .size(12.0)
        .color(theme::TEXT_DIM),
    );
}

fn section(ui: &mut egui::Ui, title: &str, files: &[FileEntry], actions: &mut Vec<UiAction>) {
    widgets::section_title(ui, title);
    if files.is_empty() {
        ui.label(egui::RichText::new("Aucun fichier").color(theme::TEXT_FAINT));
    }
    for file in files {
        let icon = match file.format {
            Format::Bones => Icon::Bone,
            Format::Bvh => Icon::Trajectory,
            _ => Icon::Import,
        };
        let badge = file.format.extension().to_uppercase();
        if widgets::list_row(ui, icon, &file.name, Some(file.format.label()), Some(&badge)).clicked() {
            actions.push(UiAction::Import(file.path.clone()));
        }
    }
}
