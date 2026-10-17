//! Fenêtres contextuelles ouvertes depuis le bloc du haut (comme les menus
//! de Procreate) : galerie, actions, affichage, os, réglages. Elles
//! apparaissent en fondu avec un léger glissement.

mod actions;
mod gallery;
mod outliner;
mod settings;
mod view;

use bones_core::Editor;
use bones_ui::icons::Icon;
use bones_ui::{theme, widgets};

use super::{EDGE, UiAction, UiState, block};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Popover {
    Gallery,
    Actions,
    View,
    Outliner,
    Settings,
}

impl Popover {
    fn title(self) -> &'static str {
        match self {
            Popover::Gallery => "Gallery",
            Popover::Actions => "Actions",
            Popover::View => "View",
            Popover::Outliner => "Bones",
            Popover::Settings => "Settings",
        }
    }
}

pub fn show(
    ctx: &egui::Context,
    editor: &mut Editor,
    state: &mut UiState,
    actions: &mut Vec<UiAction>,
) -> Option<egui::Rect> {
    let Some(popover) = state.popover else {
        // Fenêtre fermée : la prochaine ouverture rejouera l'animation.
        ctx.animate_bool_with_time(anim_id(), false, 0.0);
        return None;
    };
    let t = ctx.animate_bool_with_time(anim_id(), true, 0.16);
    let mut close = false;
    let offset = [EDGE, 76.0 - 10.0 * (1.0 - t)];
    let rect = block(
        ctx,
        "popover",
        egui::Align2::LEFT_TOP,
        offset,
        theme::panel_frame(),
        |ui| {
            ui.set_opacity(t);
            ui.set_width(384.0);
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(popover.title()).font(theme::semibold(21.0)));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if widgets::tool_button(ui, Icon::Close, false, "Close").clicked() {
                        close = true;
                    }
                });
            });
            ui.add_space(2.0);
            match popover {
                Popover::Gallery => gallery::show(ui, editor, state, actions),
                Popover::Actions => actions::show(ui, editor, actions),
                Popover::View => view::show(ui, editor),
                Popover::Outliner => outliner::show(ui, editor, state),
                Popover::Settings => settings::show(ui, state),
            }
        },
    );
    let picked = actions
        .iter()
        .any(|a| matches!(a, UiAction::Import(_) | UiAction::Export(_)));
    if close || picked {
        state.popover = None;
    }
    Some(rect)
}

fn anim_id() -> egui::Id {
    egui::Id::unique("popover-anim")
}
