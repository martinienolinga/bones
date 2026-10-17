//! Section « Tools » : copies, subdivision et suppression de l'os
//! sélectionné (raccourcis rappelés à droite de chaque ligne).

use bones_core::Editor;
use bones_ui::{theme, widgets};

use crate::ui::UiState;
use crate::ui::popovers::note;

pub fn show(ui: &mut egui::Ui, editor: &mut Editor, state: &mut UiState) {
    widgets::section_title(ui, "Copy");
    widgets::group(ui, |g| {
        if g.value_row("Duplicate", "D").clicked() {
            editor.duplicate_selected(false);
        }
        if g.value_row("Duplicate with children", "⇧D").clicked() {
            editor.duplicate_selected(true);
        }
    });

    widgets::section_title(ui, "Subdivide");
    let parts = &mut state.subdivide_parts;
    let apply = widgets::group(ui, |g| {
        g.custom(52.0, |ui| {
            ui.label(egui::RichText::new("Pieces").size(17.0).color(theme::TEXT_LIST));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let apply = widgets::chip_button(ui, "Subdivide", true).clicked();
                ui.add_space(6.0);
                widgets::stepper(ui, parts, 2..=8);
                apply
            })
            .inner
        })
    });
    if apply {
        editor.subdivide_selected(state.subdivide_parts);
    }
    note(ui, "Cuts the bone into connected pieces of equal length.");

    widgets::section_title(ui, "Delete");
    widgets::group(ui, |g| {
        if g.value_row("Delete bone", "⌫").clicked() {
            editor.delete_selected();
        }
        if g.value_row("Delete with children", "").clicked() {
            editor.delete_selected_with_children();
        }
    });
}
