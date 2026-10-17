//! Blocs du haut. À gauche, comme Procreate : « Galerie » puis les menus de
//! scène et les réglages. À droite, comme Nomad Sculpt : le mode de travail.

use bones_core::{Editor, Mode};
use bones_ui::icons::Icon;
use bones_ui::{theme, widgets};

use super::{EDGE, Popover, UiState, block};

pub fn left(ctx: &egui::Context, editor: &mut Editor, state: &mut UiState) -> egui::Rect {
    block(
        ctx,
        "top-left",
        egui::Align2::LEFT_TOP,
        [EDGE, EDGE],
        theme::bar_frame(),
        |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 4.0;
                let open = state.popover;
                if widgets::text_button(ui, "Gallery", open == Some(Popover::Gallery)).clicked() {
                    state.toggle(Popover::Gallery);
                }
                widgets::separator_dot(ui);
                for (icon, popover, hint) in [
                    (Icon::Actions, Popover::Actions, "Actions"),
                    (Icon::Eye, Popover::View, "View"),
                    (Icon::Hierarchy, Popover::Outliner, "Bones"),
                    (Icon::Settings, Popover::Settings, "Settings"),
                ] {
                    if widgets::tool_button(ui, icon, open == Some(popover), hint).clicked() {
                        state.toggle(popover);
                    }
                }
                widgets::separator_dot(ui);
                ui.add_space(4.0);
                ui.label(
                    egui::RichText::new(&editor.rig.name)
                        .font(theme::semibold(14.0))
                        .color(theme::TEXT_DIM),
                );
                ui.add_space(8.0);
            });
        },
    )
}

pub fn right(ctx: &egui::Context, editor: &mut Editor, _state: &mut UiState) -> egui::Rect {
    block(
        ctx,
        "top-right",
        egui::Align2::RIGHT_TOP,
        [-EDGE, EDGE],
        theme::bar_frame(),
        |ui| {
            // Même hauteur que le bloc de gauche (boutons de 44 pt).
            ui.vertical(|ui| {
                ui.spacing_mut().item_spacing.y = 0.0;
                const MODES: [Mode; 3] = [Mode::Pose, Mode::Edit, Mode::AutoRig];
                let mut index = MODES.iter().position(|m| *m == editor.mode).unwrap_or(0);
                ui.add_space(4.0);
                if widgets::segmented(ui, &["Pose", "Edit", "Auto-Rig"], &mut index).changed() {
                    editor.set_mode(MODES[index]);
                }
                ui.add_space(4.0);
            });
        },
    )
}
