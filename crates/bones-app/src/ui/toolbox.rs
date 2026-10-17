//! Boîte à outils de droite, à la Nomad Sculpt : une colonne de cellules
//! (icône en couleur + étiquette) qui dépend du mode courant, en groupes
//! séparés par de fins traits.

use bones_core::{EditTool, Editor, Mode, PoseTool, Side};
use bones_ui::icons::Icon;
use bones_ui::{theme, widgets};

use super::{COLUMN_TOP, EDGE, UiState, block};

pub fn show(ctx: &egui::Context, editor: &mut Editor, _state: &mut UiState) -> egui::Rect {
    block(
        ctx,
        "toolbox",
        egui::Align2::RIGHT_TOP,
        [-EDGE, COLUMN_TOP],
        theme::card_frame(),
        |ui| {
            ui.vertical_centered(|ui| {
                ui.spacing_mut().item_spacing.y = 4.0;
                match editor.mode {
                    Mode::Pose => pose_tools(ui, editor),
                    Mode::Edit => edit_tools(ui, editor),
                    Mode::AutoRig => autorig_tools(ui, editor),
                }
            });
        },
    )
}

/// Trait de séparation discret entre deux groupes d'outils.
fn divider(ui: &mut egui::Ui) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(40.0, 9.0), egui::Sense::hover());
    ui.painter().line_segment(
        [rect.left_center(), rect.right_center()],
        egui::Stroke::new(1.0, egui::Color32::from_white_alpha(22)),
    );
}

fn pose_tools(ui: &mut egui::Ui, editor: &mut Editor) {
    for (tool, icon, label) in [
        (PoseTool::Move, Icon::Move, "Déplacer"),
        (PoseTool::Rotate, Icon::Rotate, "Tourner"),
        (PoseTool::Pin, Icon::Pin, "Épingler"),
    ] {
        if widgets::tool_cell(ui, icon, label, editor.pose_tool == tool).clicked() {
            editor.pose_tool = tool;
        }
    }
    divider(ui);
    if widgets::tool_cell(ui, Icon::Mirror, "Miroir", false).clicked() {
        editor.mirror_pose();
    }
    if widgets::tool_cell(ui, Icon::Reset, "Repos", false).clicked() {
        let selected_only = editor.selected.is_some();
        editor.reset_pose(selected_only);
    }
    if !editor.rig.pins.is_empty() && widgets::tool_cell(ui, Icon::Close, "Épingles", false).clicked() {
        editor.clear_pins();
    }
    divider(ui);
    let view = &mut editor.view;
    toggle(ui, Icon::Trajectory, "Trajet", &mut view.trajectory);
    toggle(ui, Icon::Ghost, "Fantômes", &mut view.ghosts);
    toggle(ui, Icon::CenterOfMass, "Masse", &mut view.center_of_mass);
}

fn edit_tools(ui: &mut egui::Ui, editor: &mut Editor) {
    for (tool, icon, label) in [
        (EditTool::Select, Icon::Select, "Ajuster"),
        (EditTool::AddBone, Icon::BoneAdd, "Ajouter"),
    ] {
        if widgets::tool_cell(ui, icon, label, editor.edit_tool == tool).clicked() {
            editor.edit_tool = tool;
        }
    }
    divider(ui);
    let has_selection = editor.selected.is_some();
    if ui
        .add_enabled_ui(has_selection, |ui| {
            widgets::tool_cell(ui, Icon::Delete, "Supprimer", false)
        })
        .inner
        .clicked()
    {
        editor.delete_selected();
    }
    if widgets::tool_cell(ui, Icon::Symmetrize, "G → D", false).clicked() {
        editor.symmetrize(Side::Left);
    }
    if widgets::tool_cell(ui, Icon::Mirror, "D → G", false).clicked() {
        editor.symmetrize(Side::Right);
    }
    divider(ui);
    if widgets::tool_cell(ui, Icon::Weights, "Lier", false).clicked() {
        editor.request_rebind();
    }
}

fn autorig_tools(ui: &mut egui::Ui, editor: &mut Editor) {
    widgets::tool_cell(ui, Icon::Marker, "Marqueurs", true);
    if widgets::tool_cell(ui, Icon::Reset, "Estimer", false).clicked() {
        editor.guess_markers();
    }
    divider(ui);
    if widgets::tool_cell(ui, Icon::AutoRig, "Générer", false).clicked() {
        editor.generate_rig();
    }
}

fn toggle(ui: &mut egui::Ui, icon: Icon, label: &str, value: &mut bool) {
    if widgets::tool_cell(ui, icon, label, *value).clicked() {
        *value = !*value;
    }
}
