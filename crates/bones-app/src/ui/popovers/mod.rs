//! Fenêtres contextuelles ouvertes depuis le bloc du haut (comme les menus
//! de Procreate) : galerie, actions, affichage, os.

mod actions;
mod gallery;
mod outliner;
mod view;

use bones_core::Editor;
use bones_ui::theme;

use super::{UiAction, UiState, block};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Popover {
    Gallery,
    Actions,
    View,
    Outliner,
}

impl Popover {
    fn title(self) -> &'static str {
        match self {
            Popover::Gallery => "Galerie",
            Popover::Actions => "Actions",
            Popover::View => "Affichage",
            Popover::Outliner => "Os et membres",
        }
    }
}

pub fn show(
    ctx: &egui::Context,
    editor: &mut Editor,
    state: &mut UiState,
    actions: &mut Vec<UiAction>,
) -> Option<egui::Rect> {
    let popover = state.popover?;
    let mut close = false;
    let rect = block(
        ctx,
        "popover",
        egui::Align2::LEFT_TOP,
        [8.0, 70.0],
        theme::panel_frame(),
        |ui| {
            ui.set_width(360.0);
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(popover.title()).strong().size(19.0));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if bones_ui::widgets::tool_button(ui, bones_ui::icons::Icon::Close, false, "Fermer")
                        .clicked()
                    {
                        close = true;
                    }
                });
            });
            ui.separator();
            match popover {
                Popover::Gallery => gallery::show(ui, editor, state, actions),
                Popover::Actions => actions::show(ui, editor, state, actions),
                Popover::View => view::show(ui, editor),
                Popover::Outliner => outliner::show(ui, editor, state),
            }
        },
    );
    if close
        || actions
            .iter()
            .any(|a| matches!(a, UiAction::Import(_) | UiAction::Export(_)))
    {
        state.popover = None;
    }
    Some(rect)
}

/// Bouton pleine largeur d'une liste.
fn row_button(ui: &mut egui::Ui, text: &str) -> egui::Response {
    ui.add_sized([ui.available_width(), 40.0], egui::Button::new(text))
}
