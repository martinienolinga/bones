//! Blocs du haut. À gauche, comme Procreate : « Galerie » puis les menus de
//! scène. À droite, comme Nomad Sculpt : le choix du mode de travail.

use bones_core::{Editor, Mode};
use bones_ui::icons::Icon;
use bones_ui::{theme, widgets};

use super::{Popover, UiState, block};

const MARGIN: f32 = 8.0;

pub fn left(ctx: &egui::Context, editor: &mut Editor, state: &mut UiState) -> egui::Rect {
    block(
        ctx,
        "top-left",
        egui::Align2::LEFT_TOP,
        [MARGIN, MARGIN],
        theme::bar_frame(),
        |ui| {
            ui.horizontal(|ui| {
                let open = state.popover;
                if widgets::text_button(ui, "Galerie", open == Some(Popover::Gallery)).clicked() {
                    state.toggle(Popover::Gallery);
                }
                ui.add_space(4.0);
                for (icon, popover, hint) in [
                    (Icon::Actions, Popover::Actions, "Actions"),
                    (Icon::Eye, Popover::View, "Affichage"),
                    (Icon::Hierarchy, Popover::Outliner, "Os et membres"),
                ] {
                    if widgets::tool_button(ui, icon, open == Some(popover), hint).clicked() {
                        state.toggle(popover);
                    }
                }
                ui.add_space(6.0);
                ui.label(egui::RichText::new(&editor.rig.name).color(theme::TEXT_DIM));
            });
        },
    )
}

pub fn right(ctx: &egui::Context, editor: &mut Editor, _state: &mut UiState) -> egui::Rect {
    block(
        ctx,
        "top-right",
        egui::Align2::RIGHT_TOP,
        [-MARGIN, MARGIN],
        theme::bar_frame(),
        |ui| {
            ui.horizontal(|ui| {
                const MODES: [Mode; 3] = [Mode::Pose, Mode::Edit, Mode::AutoRig];
                let mut index = MODES.iter().position(|m| *m == editor.mode).unwrap_or(0);
                if widgets::segmented(ui, &["Pose", "Édition", "Auto-Rig"], &mut index).changed() {
                    editor.set_mode(MODES[index]);
                }
            });
        },
    )
}
