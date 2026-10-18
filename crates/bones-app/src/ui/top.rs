//! Barre du haut, pleine largeur comme dans Procreate. À gauche :
//! « Gallery » (retour à la galerie des projets), puis les menus dans des
//! puits ronds (Actions, Vue, Os, Réglages) et le nom du projet. À droite : les outils du mode courant
//! (glyphes bleus quand actifs) et le choix du mode.

use bones_core::{EditTool, Editor, Mode, PoseTool};
use bones_ui::icons::Icon;
use bones_ui::{theme, widgets};

use super::{BAR_H, Popover, UiState};

pub fn show(ctx: &egui::Context, editor: &mut Editor, state: &mut UiState) -> egui::Rect {
    let screen = ctx.content_rect();
    let rect = egui::Rect::from_min_size(screen.min, egui::vec2(screen.width(), BAR_H));
    egui::Area::new(egui::Id::unique("top-bar"))
        .fixed_pos(rect.min)
        .order(egui::Order::Foreground)
        .show(ctx, |ui| {
            ui.painter().rect_filled(rect, 0, theme::BAR);
            ui.allocate_rect(rect, egui::Sense::hover());
            let inner = rect.shrink2(egui::vec2(10.0, 0.0));
            let mut left = ui.new_child(
                egui::UiBuilder::new()
                    .max_rect(inner)
                    .layout(egui::Layout::left_to_right(egui::Align::Center)),
            );
            menus(&mut left, editor, state);
            let mut right = ui.new_child(
                egui::UiBuilder::new()
                    .max_rect(inner)
                    .layout(egui::Layout::right_to_left(egui::Align::Center)),
            );
            modes(&mut right, editor);
            right.add_space(12.0);
            tools(&mut right, editor, state);
        });
    rect
}

fn menus(ui: &mut egui::Ui, editor: &Editor, state: &mut UiState) {
    ui.spacing_mut().item_spacing.x = 4.0;
    let open = state.popover;
    if widgets::text_button(ui, "Gallery", false).clicked() {
        state.gallery_requested = true;
    }
    ui.add_space(8.0);
    for (icon, popover, hint) in [
        (Icon::Actions, Popover::Actions, "Actions"),
        (Icon::Eye, Popover::View, "View"),
        (Icon::Bone, Popover::Outliner, "Bones"),
        (Icon::Settings, Popover::Settings, "Settings"),
    ] {
        let button = widgets::well_button(ui, icon, open == Some(popover), hint);
        if button.clicked() {
            state.toggle(popover, button.rect.center().x);
        }
    }
    ui.add_space(14.0);
    ui.label(
        egui::RichText::new(&editor.rig.name)
            .size(15.0)
            .color(theme::TEXT_DIM),
    );
}

/// Outils du mode, de droite à gauche (la mise en page est inversée).
fn tools(ui: &mut egui::Ui, editor: &mut Editor, state: &mut UiState) {
    ui.spacing_mut().item_spacing.x = 4.0;
    match editor.mode {
        Mode::WeightPaint => {
            let open = state.popover;
            // Menu des outils de poids (le plus à droite), puis la
            // bibliothèque de pinceaux (glyphe de l'outil actif).
            let tools = widgets::bar_tool(
                ui,
                Icon::Normalize,
                open == Some(Popover::WeightsTools),
                "Weights tools",
            );
            if tools.clicked() {
                state.toggle(Popover::WeightsTools, tools.rect.center().x);
            }
            let brush_icon = super::popovers::brush_tool_icon(editor.weight_paint.brush.tool);
            let brush = widgets::bar_tool(ui, brush_icon, open == Some(Popover::Brush), "Brush library");
            if brush.clicked() {
                state.toggle(Popover::Brush, brush.rect.center().x);
            }
        }
        Mode::Pose => {
            let open = state.popover;
            let tools = [
                (PoseTool::Move, Icon::Move, "Move (IK)"),
                (PoseTool::Rotate, Icon::Rotate, "Rotate"),
                (PoseTool::Pin, Icon::Pin, "Pin"),
            ];
            for (tool, icon, hint) in tools.into_iter().rev() {
                if widgets::bar_tool(ui, icon, editor.pose_tool == tool, hint).clicked() {
                    editor.pose_tool = tool;
                }
            }
            widgets::separator_dot(ui);
            // Bibliothèque d'animations, et expressions si le rig a un visage.
            let library = widgets::bar_tool(
                ui,
                Icon::Motion,
                open == Some(Popover::Presets),
                "Animation library",
            );
            if library.clicked() {
                state.toggle(Popover::Presets, library.rect.center().x);
            }
            if bones_core::autorig::face::present(&editor.rig.skeleton).len() >= 2 {
                let expr = widgets::bar_tool(
                    ui,
                    Icon::Face,
                    open == Some(Popover::Expressions),
                    "Facial expressions",
                );
                if expr.clicked() {
                    state.toggle(Popover::Expressions, expr.rect.center().x);
                }
            }
        }
        Mode::Edit => {
            let tools = [
                (EditTool::Select, Icon::Select, "Adjust: drag a tail"),
                (EditTool::AddBone, Icon::BoneAdd, "Add: drag out a new bone"),
                (EditTool::MoveHead, Icon::BoneHead, "Head: drag a head"),
                (EditTool::MoveBone, Icon::Move, "Move: drag a whole bone"),
                (EditTool::Roll, Icon::Rotate, "Roll: twist around the bone axis"),
                (
                    EditTool::Reparent,
                    Icon::Hierarchy,
                    "Parent: tap a bone, then its new parent",
                ),
            ];
            for (tool, icon, hint) in tools.into_iter().rev() {
                if widgets::bar_tool(ui, icon, editor.edit_tool == tool, hint).clicked() {
                    editor.edit_tool = tool;
                }
            }
        }
        Mode::AutoRig => {
            widgets::bar_tool(ui, Icon::Marker, true, "Markers: drag them onto the model");
        }
        // Outils du mode Objet : étape 3 (interface).
        Mode::Object => {}
    }
}

fn modes(ui: &mut egui::Ui, editor: &mut Editor) {
    const MODES: [Mode; 5] = [
        Mode::Pose,
        Mode::WeightPaint,
        Mode::Edit,
        Mode::Object,
        Mode::AutoRig,
    ];
    let mut index = MODES.iter().position(|m| *m == editor.mode).unwrap_or(0);
    // Le contrôle dessine ses segments de gauche à droite dans sa zone.
    if widgets::segmented(ui, &["Pose", "Weights", "Edit", "Object", "Auto-Rig"], &mut index).changed() {
        editor.set_mode(MODES[index]);
    }
}
