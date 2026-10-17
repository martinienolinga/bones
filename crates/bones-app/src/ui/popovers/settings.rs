//! Studio des réglages : tâches réservées à l'Apple Pencil, guide de
//! rotation et rayon de sélection tactile. Chaque changement s'applique tout
//! de suite ; « Cancel » rétablit les réglages d'avant l'ouverture.

use bones_ui::icons::Icon;
use bones_ui::widgets;

use super::{note, studio};
use crate::gizmo::GizmoStyle;
use crate::settings::PencilPolicy;
use crate::ui::UiState;

const SECTIONS: [(Icon, &str); 3] = [
    (Icon::Edit, "Apple Pencil"),
    (Icon::Ortho, "Rotation guide"),
    (Icon::Select, "Touch"),
];

/// Retourne vrai quand le studio doit se fermer.
pub fn show(ui: &mut egui::Ui, state: &mut UiState) -> bool {
    let mut close = false;
    let mut section = state.settings_section;
    let mut cancel = false;
    studio::show(
        ui,
        "Settings",
        &SECTIONS,
        &mut section,
        440.0,
        |ui| {
            close = widgets::confirm_button(ui, "Done").clicked();
            ui.add_space(4.0);
            cancel = widgets::cancel_button(ui, "Cancel").clicked();
        },
        |ui, section| match section {
            0 => pencil(ui, state),
            1 => guide(ui, state),
            _ => touch(ui, state),
        },
    );
    state.settings_section = section;
    if cancel {
        state.settings = state.settings_backup;
        state.settings_dirty = true;
    }
    close || cancel
}

fn pencil(ui: &mut egui::Ui, state: &mut UiState) {
    widgets::section_title(ui, "Apple Pencil only for");
    let changed = widgets::group(ui, |g| {
        let mut changed = false;
        for (index, (label, description)) in PencilPolicy::TASKS.iter().enumerate() {
            let value = state.settings.pencil.get_mut(index);
            changed |= g.switch_row(value, label, Some(description)).changed();
        }
        changed
    });
    state.settings_dirty |= changed;
    note(ui, "Fingers then only navigate (orbit, pinch, undo taps).");
}

fn guide(ui: &mut egui::Ui, state: &mut UiState) {
    widgets::section_title(ui, "Style");
    let current = state.settings.gizmo;
    let picked = widgets::group(ui, |g| {
        let mut picked = None;
        for (style, label) in [
            (GizmoStyle::Axes, "Axes (Blender style)"),
            (GizmoStyle::Cube, "Cube (Nomad style)"),
        ] {
            if g.check_row(label, current == style).clicked() {
                picked = Some(style);
            }
        }
        picked
    });
    if let Some(style) = picked.filter(|s| *s != current) {
        state.settings.gizmo = style;
        state.settings_dirty = true;
    }
    note(ui, "Tap a face or an axis to look along it; drag to orbit.");
}

fn touch(ui: &mut egui::Ui, state: &mut UiState) {
    let mut radius = state.settings.pick_radius;
    if widgets::slider_row(ui, &mut radius, 10.0..=48.0, "Selection radius", "pt").changed() {
        state.settings.pick_radius = radius.round();
        state.settings_dirty = true;
    }
    note(
        ui,
        "How close, in points, a finger must be to grab a bone or a handle.",
    );
}
