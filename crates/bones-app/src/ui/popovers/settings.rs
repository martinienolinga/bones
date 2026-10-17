//! Réglages : tâches réservées à l'Apple Pencil (les doigts naviguent),
//! guide de rotation et rayon de sélection tactile. Chaque changement marque
//! les réglages comme à enregistrer.

use bones_ui::{theme, widgets};

use super::UiState;
use crate::gizmo::GizmoStyle;
use crate::settings::PencilPolicy;

pub fn show(ui: &mut egui::Ui, state: &mut UiState) {
    widgets::section_title(ui, "Apple Pencil uniquement pour :");
    ui.spacing_mut().item_spacing.y = 6.0;
    for (index, (label, description)) in PencilPolicy::TASKS.iter().enumerate() {
        let value = state.settings.pencil.get_mut(index);
        if widgets::switch_row(ui, value, label, description).changed() {
            state.settings_dirty = true;
        }
    }
    ui.label(
        egui::RichText::new(
            "Les doigts servent alors uniquement à naviguer (orbite, pincement, tapes d'annulation).",
        )
        .size(12.0)
        .color(theme::TEXT_DIM),
    );
    ui.add_space(10.0);
    widgets::section_title(ui, "Vue");
    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            ui.spacing_mut().item_spacing.y = 0.0;
            ui.label("Guide de rotation");
            ui.label(
                egui::RichText::new("axes à la Blender ou cube à la Nomad")
                    .size(12.0)
                    .color(theme::TEXT_DIM),
            );
        });
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            const STYLES: [GizmoStyle; 2] = [GizmoStyle::Axes, GizmoStyle::Cube];
            let mut index = STYLES
                .iter()
                .position(|s| *s == state.settings.gizmo)
                .unwrap_or(0);
            if widgets::segmented(ui, &["Axes", "Cube"], &mut index).changed() {
                state.settings.gizmo = STYLES[index];
                state.settings_dirty = true;
            }
        });
    });
    ui.add_space(10.0);
    widgets::section_title(ui, "Toucher");
    let mut radius = state.settings.pick_radius;
    if widgets::slider_row(ui, &mut radius, 10.0..=48.0, "Rayon de sélection", "pt").changed() {
        state.settings.pick_radius = radius.round();
        state.settings_dirty = true;
    }
    ui.label(
        egui::RichText::new("Distance, en points, à laquelle un os ou une poignée réagit au doigt.")
            .size(12.0)
            .color(theme::TEXT_DIM),
    );
}
