//! Fenêtres contextuelles ouvertes depuis la barre du haut, comme celles de
//! Procreate : panneau #2C2C2E aux coins de 20 pt, large ombre légère et
//! encoche pointée vers le bouton d'origine. Actions et Vue se
//! ferment d'un toucher à l'extérieur ; Réglages et Os sont des « studios »
//! (navigation à gauche, valeurs à droite) fermés par leurs boutons.

mod actions;
mod bones;
mod brush;
mod settings;
mod studio;
mod view;
mod weights_tools;

use bones_core::Editor;
use bones_ui::icons::Icon;
use bones_ui::{theme, widgets};

pub use brush::tool_icon as brush_tool_icon;
pub use weights_tools::{State as WeightsToolsState, confirm_dialog as confirm_automatic};

use super::{BAR_H, EDGE, UiAction, UiState};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Popover {
    Actions,
    View,
    Outliner,
    Settings,
    /// Bibliothèque de pinceaux (studio du mode Weight Paint).
    Brush,
    /// Menu des outils du menu « Weights » de Blender.
    WeightsTools,
}

impl Popover {
    fn width(self) -> f32 {
        match self {
            Popover::Outliner => 720.0,
            Popover::Settings | Popover::Brush => 640.0,
            Popover::Actions => 420.0,
            Popover::View => 400.0,
            Popover::WeightsTools => 440.0,
        }
    }

    fn is_studio(self) -> bool {
        matches!(self, Popover::Outliner | Popover::Settings | Popover::Brush)
    }
}

/// Hauteur de l'encoche au-dessus du panneau.
const NOTCH: f32 = 10.0;

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
    let screen = ctx.content_rect();
    let width = popover.width();
    let left = (state.popover_anchor - 44.0).clamp(EDGE, (screen.right() - width - EDGE).max(EDGE));
    let top = BAR_H + NOTCH + 4.0 - 8.0 * (1.0 - t);
    let max_height = screen.height() - top - 24.0;
    let margin = if popover.is_studio() { 10 } else { 20 };
    let mut close = false;
    let area = egui::Area::new(egui::Id::unique("popover"))
        .fixed_pos(egui::pos2(left, top))
        .order(egui::Order::Foreground)
        .show(ctx, |ui| {
            ui.set_opacity(t);
            let frame = theme::panel_frame().inner_margin(egui::Margin::same(margin));
            let panel = frame.show(ui, |ui| {
                ui.set_width(width - 2.0 * margin as f32);
                // Une `Area` borne son contenu à sa taille de l'image
                // précédente : sans cela, les listes défilantes ne
                // pourraient jamais grandir.
                ui.set_max_height(max_height - 2.0 * margin as f32);
                match popover {
                    Popover::Actions => actions::show(ui, editor, state, actions, max_height),
                    Popover::View => view::show(ui, editor, max_height),
                    Popover::Outliner => close = bones::show(ui, editor, state, studio_height(max_height)),
                    Popover::Settings => close = settings::show(ui, state, studio_height(max_height) - 120.0),
                    Popover::Brush => close = brush::show(ui, editor, state, studio_height(max_height)),
                    Popover::WeightsTools => close = weights_tools::show(ui, editor, state, max_height),
                }
            });
            let rect = panel.response.rect;
            let x = state
                .popover_anchor
                .clamp(rect.left() + 30.0, rect.right() - 30.0);
            ui.painter().add(egui::Shape::convex_polygon(
                vec![
                    egui::pos2(x - 12.0, rect.top() + 1.0),
                    egui::pos2(x, rect.top() - NOTCH),
                    egui::pos2(x + 12.0, rect.top() + 1.0),
                ],
                theme::PANEL,
                egui::Stroke::NONE,
            ));
            rect
        });
    let rect = area.inner.with_min_y(area.inner.top() - NOTCH);

    let tapped_outside = ctx.input(|i| {
        i.pointer.any_pressed()
            && i.pointer
                .interact_pos()
                .is_some_and(|p| !rect.contains(p) && p.y > BAR_H)
    });
    let picked = actions.iter().any(|a| matches!(a, UiAction::Export(_)));
    if close || picked || (tapped_outside && !popover.is_studio()) {
        state.popover = None;
    }
    Some(rect)
}

/// Hauteur d'un studio : 620 pt au plus, moins sur les petits écrans.
fn studio_height(max_height: f32) -> f32 {
    (max_height - 20.0).clamp(360.0, 620.0)
}

fn anim_id() -> egui::Id {
    egui::Id::unique("popover-anim")
}

/// Ligne de titre : 24 pt semi-gras, bouton « + » facultatif à droite.
fn header(ui: &mut egui::Ui, title: &str, plus: Option<&str>) -> bool {
    let mut clicked = false;
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new(title).font(theme::semibold(24.0)));
        if let Some(hint) = plus {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                clicked = widgets::tool_button(ui, Icon::Plus, false, hint).clicked();
            });
        }
    });
    clicked
}

/// Note discrète sous une liste (13 pt, retrait de 16 pt).
fn note(ui: &mut egui::Ui, text: &str) {
    egui::Frame::new()
        .inner_margin(egui::Margin {
            left: 16,
            right: 16,
            top: 2,
            bottom: 2,
        })
        .show(ui, |ui| {
            ui.label(egui::RichText::new(text).size(13.0).color(theme::TEXT_DIM));
        });
}
