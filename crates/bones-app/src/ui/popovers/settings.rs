//! Réglages : tâches réservées à l'Apple Pencil (les doigts naviguent),
//! guide de rotation et rayon de sélection tactile. Chaque changement marque
//! les réglages comme à enregistrer.

use bones_ui::{theme, widgets};

use super::UiState;
use crate::gizmo::GizmoStyle;
use crate::settings::PencilPolicy;

pub fn show(ui: &mut egui::Ui, state: &mut UiState) {
    widgets::section_title(ui, "Apple Pencil only for:");
    ui.spacing_mut().item_spacing.y = 6.0;
    for (index, (label, description)) in PencilPolicy::TASKS.iter().enumerate() {
        let value = state.settings.pencil.get_mut(index);
        if widgets::switch_row(ui, value, label, description).changed() {
            state.settings_dirty = true;
        }
    }
    ui.label(
        egui::RichText::new("Fingers then only navigate (orbit, pinch, undo taps).")
            .size(12.0)
            .color(theme::TEXT_DIM),
    );
    ui.add_space(10.0);
    widgets::section_title(ui, "View");
    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            ui.spacing_mut().item_spacing.y = 0.0;
            ui.label("Rotation guide");
            ui.label(
                egui::RichText::new("Blender-style axes or Nomad-style cube")
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
    widgets::section_title(ui, "Touch");
    let mut radius = state.settings.pick_radius;
    if widgets::slider_row(ui, &mut radius, 10.0..=48.0, "Selection radius", "pt").changed() {
        state.settings.pick_radius = radius.round();
        state.settings_dirty = true;
    }
    ui.label(
        egui::RichText::new("How close, in points, a finger must be to grab a bone or handle.")
            .size(12.0)
            .color(theme::TEXT_DIM),
    );
}
