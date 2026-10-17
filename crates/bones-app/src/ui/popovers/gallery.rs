//! Galerie : nouveau mannequin (« + »), nom et enregistrement du document,
//! puis les fichiers du dossier Documents (visible dans l'app Fichiers de
//! l'iPad) — documents `.bones` à ouvrir et modèles 3D à importer.

use bones_core::Editor;
use bones_core::io::Format;
use bones_ui::icons::Icon;
use bones_ui::widgets;

use super::{header, note};
use crate::files::FileEntry;
use crate::ui::{UiAction, UiState};

pub fn show(
    ui: &mut egui::Ui,
    editor: &mut Editor,
    state: &mut UiState,
    actions: &mut Vec<UiAction>,
    max_height: f32,
) {
    if header(ui, "Gallery", Some("New mannequin")) {
        actions.push(UiAction::NewDemo);
    }
    ui.add_space(6.0);
    ui.horizontal(|ui| {
        let save_width = 96.0;
        ui.add(
            egui::TextEdit::singleline(&mut editor.rig.name)
                .hint_text("Name")
                .font(egui::FontId::proportional(17.0))
                .desired_width(ui.available_width() - save_width - 8.0)
                .margin(egui::Margin::symmetric(14, 10)),
        );
        if widgets::pill_button(ui, Some(Icon::Save), "Save", true).clicked() {
            actions.push(UiAction::Save);
        }
    });
    ui.add_space(4.0);

    let (documents, models) = state.files().clone();
    egui::ScrollArea::vertical()
        .max_height((max_height - 190.0).max(160.0))
        .show(ui, |ui| {
            section(ui, "Documents", &documents, actions);
            ui.add_space(4.0);
            section(ui, "Import", &models, actions);
        });
    ui.add_space(6.0);
    note(
        ui,
        "Put your files (glTF, GLB, FBX, OBJ + MTL, STL, PLY, BVH) and their textures in \
         “On My iPad › Bones” using the Files app. Exports go to “Exports”.",
    );
}

fn section(ui: &mut egui::Ui, title: &str, files: &[FileEntry], actions: &mut Vec<UiAction>) {
    widgets::section_title(ui, title);
    widgets::group(ui, |g| {
        if files.is_empty() {
            g.value_row("No files", "");
        }
        for file in files {
            let icon = match file.format {
                Format::Bones => Icon::Bone,
                Format::Bvh => Icon::Trajectory,
                _ => Icon::Import,
            };
            let badge = file.format.extension().to_uppercase();
            // « Binary glTF (Blender, Unity…) » : seul le nom du format.
            let label = file.format.label();
            let kind = label.split_once(" (").map_or(label, |(kind, _)| kind);
            if g.icon_row(icon, &file.name, Some(kind), Some(&badge)).clicked() {
                actions.push(UiAction::Import(file.path.clone()));
            }
        }
    });
}
