//! Actions : saisie, liaison des poids, animation, export.

use bones_core::io::Format;
use bones_core::{BindMethod, Editor};
use bones_ui::{theme, widgets};

use super::{UiAction, UiState, row_button};

pub fn show(ui: &mut egui::Ui, editor: &mut Editor, state: &mut UiState, actions: &mut Vec<UiAction>) {
    ui.label(egui::RichText::new("Saisie").strong());
    ui.checkbox(
        &mut state.pencil_only,
        "Apple Pencil pour manipuler (les doigts naviguent)",
    );
    ui.add_space(10.0);

    ui.label(egui::RichText::new("Poids automatiques").strong());
    let methods = [BindMethod::Heat, BindMethod::Envelope];
    let mut index = methods.iter().position(|m| *m == editor.bind_method).unwrap_or(0);
    if widgets::segmented(ui, &["Diffusion de chaleur", "Enveloppes"], &mut index).changed() {
        editor.bind_method = methods[index];
        editor.request_rebind();
    }
    ui.label(
        egui::RichText::new(match editor.bind_method {
            BindMethod::Heat => "Qualité (Baran & Popović 2007), calculé en tâche de fond.",
            BindMethod::Envelope => "Instantané, selon la distance aux os.",
        })
        .small()
        .color(theme::TEXT_DIM),
    );
    if row_button(ui, "Recalculer les poids").clicked() {
        editor.request_rebind();
    }

    ui.add_space(10.0);
    ui.label(egui::RichText::new("Animation").strong());
    ui.horizontal(|ui| {
        ui.label("Images par seconde");
        let clip = &editor.rig.clip;
        let mut fps = clip.fps;
        if ui
            .add(egui::DragValue::new(&mut fps).range(1.0..=120.0))
            .changed()
        {
            editor.rig.clip_mut().fps = fps;
        }
    });

    ui.add_space(10.0);
    export(ui, actions);
}

/// Partage façon Procreate : un bouton par format, avec ce qu'il emporte.
fn export(ui: &mut egui::Ui, actions: &mut Vec<UiAction>) {
    ui.label(egui::RichText::new("Exporter").strong());
    for format in Format::exportable().filter(|f| *f != Format::Bones) {
        let mut contents = vec!["maillage"];
        if format.supports_textures() {
            contents.push("textures");
        }
        if format.exports_skeleton() {
            contents.push("squelette");
        }
        if format.exports_animation() {
            contents.push("animation");
        }
        let text = format!(".{}  —  {}", format.extension(), format.label());
        let hint = format!("Contient : {}", contents.join(", "));
        if row_button(ui, &text).on_hover_text(hint).clicked() {
            actions.push(UiAction::Export(format));
        }
    }
    ui.label(
        egui::RichText::new("Les formats sans squelette (OBJ, STL, PLY) exportent la pose courante.")
            .small()
            .color(theme::TEXT_DIM),
    );
}
