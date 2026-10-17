//! Actions : liaison des poids, animation, export.

use bones_core::{BindMethod, Editor};
use bones_ui::{theme, widgets};

use super::{UiAction, row_button};

pub fn show(ui: &mut egui::Ui, editor: &mut Editor, actions: &mut Vec<UiAction>) {
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
    ui.label(egui::RichText::new("Exporter").strong());
    if row_button(ui, "glTF binaire (.glb) — Blender, Unity, Unreal, Godot").clicked() {
        actions.push(UiAction::ExportGlb);
    }
}
